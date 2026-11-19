//! Port allocation module for managing dynamic port assignment.
//!
//! This module provides a `PortAllocator` that manages a contiguous range of ports
//! for the devnet cluster. All components (Lotus, Lotus-Miner, Curio, the
//! Postgres/Scylla databases) dynamically allocate ports from this pool,
//! ensuring no conflicts.

use std::collections::HashSet;
use std::error::Error;

use crate::docker::core::is_port_available;

/// Manages dynamic allocation of ports from a contiguous range.
///
/// The allocator tracks which ports have been assigned and ensures
/// that each allocation is unique, within the configured range, and
/// actually free on the host at allocation time.
#[derive(Debug)]
pub struct PortAllocator {
    /// Starting port of the range
    start: u16,

    /// Last (inclusive) port of the range.
    ///
    /// Stored instead of a count so that no code path has to compute
    /// `start + count`, which would overflow u16 for ranges ending at 65535.
    end: u16,

    /// Set of already allocated ports
    allocated: HashSet<u16>,

    /// Next port to try allocating
    next: u16,
}

impl PortAllocator {
    /// Create a new PortAllocator with the specified range.
    ///
    /// # Arguments
    ///
    /// * `start` - The first port in the range (e.g., 5700)
    /// * `count` - Number of ports in the range (e.g., 300 for ports 5700-5999)
    ///
    /// # Returns
    ///
    /// A new PortAllocator instance, or an error if the range is invalid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `count` is 0
    /// - `start` is 0 (port 0 means "pick any port" and cannot be published)
    /// - The range would overflow u16 (start + count - 1 > 65535)
    pub fn new(start: u16, count: u16) -> Result<Self, Box<dyn Error>> {
        if count == 0 {
            return Err("Port range count must be greater than 0".into());
        }

        if start == 0 {
            return Err("Port range must not start at port 0".into());
        }

        // `count` is non-zero, so `count - 1` is safe; the sum is checked.
        let end = start
            .checked_add(count - 1)
            .ok_or("Port range exceeds maximum port number (65535)")?;

        Ok(Self {
            start,
            end,
            allocated: HashSet::new(),
            next: start,
        })
    }

    /// Number of ports in the range.
    fn size(&self) -> u16 {
        // `start >= 1`, so `end - start + 1` can never overflow.
        self.end - self.start + 1
    }

    /// Allocate a single port from the range.
    ///
    /// This method finds the next available port in the range and marks it as allocated.
    /// Ports already bound on the host are skipped (and permanently marked allocated) so
    /// that the allocator never hands out a port that is in use by another process.
    ///
    /// # Returns
    ///
    /// The allocated port number, or an error if no ports are available.
    ///
    /// # Errors
    ///
    /// Returns an error if all ports in the range are already allocated or in use.
    pub fn allocate(&mut self) -> Result<u16, Box<dyn Error>> {
        let mut in_use = Vec::new();

        for _ in 0..self.size() {
            let port = self.next;

            // Advance next port (wrap around at the end of the range)
            self.next = if port == self.end {
                self.start
            } else {
                port + 1
            };

            // Check if this port is already allocated
            if self.allocated.contains(&port) {
                continue;
            }

            if !is_port_available(port) {
                // Reserve it so we neither return it nor retry it on the next call.
                self.allocated.insert(port);
                in_use.push(port);
                continue;
            }

            self.allocated.insert(port);
            return Ok(port);
        }

        let mut msg = format!("No available ports in range {}-{}", self.start, self.end);
        if !in_use.is_empty() {
            msg.push_str(&format!(
                " (in use by other processes: {})",
                in_use
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Err(msg.into())
    }

    /// Allocate multiple ports from the range.
    pub fn allocate_multiple(&mut self, count: usize) -> Result<Vec<u16>, Box<dyn Error>> {
        let mut ports = Vec::with_capacity(count);
        for _ in 0..count {
            ports.push(self.allocate()?);
        }
        Ok(ports)
    }

    /// Mark a specific port as allocated.
    ///
    /// # Arguments
    ///
    /// * `port` - The port to mark as allocated
    ///
    /// # Errors
    ///
    /// Returns an error if the port is outside the managed range.
    pub fn mark_allocated(&mut self, port: u16) -> Result<(), Box<dyn Error>> {
        if port < self.start || port > self.end {
            return Err(format!(
                "Port {} is outside the range {}-{}",
                port, self.start, self.end
            )
            .into());
        }
        self.allocated.insert(port);
        Ok(())
    }

    /// Get the number of allocated ports.
    pub fn allocated_count(&self) -> usize {
        self.allocated.len()
    }

    /// Get the number of remaining available ports.
    pub fn available_count(&self) -> usize {
        self.size() as usize - self.allocated.len()
    }

    /// Get the port range (start, end), both inclusive.
    pub fn range(&self) -> (u16, u16) {
        (self.start, self.end)
    }

    /// Check if all ports in the range are available on the system.
    ///
    /// This performs a pre-flight check to ensure no other processes
    /// are using ports in the configured range.
    ///
    /// # Returns
    ///
    /// Ok(()) if all ports are available, or an error listing the ports that are in use.
    pub fn verify_all_ports_available(&self) -> Result<(), Box<dyn Error>> {
        let mut unavailable_ports = Vec::new();

        for port in self.start..=self.end {
            if !is_port_available(port) {
                unavailable_ports.push(port);
            }
        }

        if !unavailable_ports.is_empty() {
            return Err(format!(
                "The following {} port(s) in the configured range are already in use: {}\n\
                Please either:\n\
                1. Stop the processes using these ports, or\n\
                2. Configure a different port range in ~/.foc-devnet/config.toml",
                unavailable_ports.len(),
                format_port_list(&unavailable_ports)
            )
            .into());
        }

        Ok(())
    }
}

/// Format a list of ports for display in error messages.
///
/// Formats up to 10 ports, showing "... and N more" if there are more.
fn format_port_list(ports: &[u16]) -> String {
    const MAX_DISPLAY: usize = 10;

    if ports.len() <= MAX_DISPLAY {
        ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        let displayed: Vec<String> = ports[..MAX_DISPLAY].iter().map(|p| p.to_string()).collect();
        format!(
            "{} ... and {} more",
            displayed.join(", "),
            ports.len() - MAX_DISPLAY
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn test_allocator_creation() {
        let allocator = PortAllocator::new(5700, 300).unwrap();
        assert_eq!(allocator.range(), (5700, 5999));
        assert_eq!(allocator.available_count(), 300);
        assert_eq!(allocator.allocated_count(), 0);
    }

    // Allocation probes the host, so each allocating test uses its own range
    // below the ephemeral port range to stay independent of other tests.
    #[test]
    fn test_allocate_single() {
        let mut allocator = PortAllocator::new(15800, 10).unwrap();
        let port1 = allocator.allocate().unwrap();
        let port2 = allocator.allocate().unwrap();

        assert_eq!(port1, 15800);
        assert_eq!(port2, 15801);
        assert_eq!(allocator.allocated_count(), 2);
        assert_eq!(allocator.available_count(), 8);
    }

    #[test]
    fn test_allocate_multiple() {
        let mut allocator = PortAllocator::new(15820, 20).unwrap();
        let ports = allocator.allocate_multiple(5).unwrap();

        assert_eq!(ports.len(), 5);
        assert_eq!(ports, vec![15820, 15821, 15822, 15823, 15824]);
        assert_eq!(allocator.allocated_count(), 5);
    }

    #[test]
    fn test_allocate_exhaustion() {
        let mut allocator = PortAllocator::new(15850, 3).unwrap();

        assert!(allocator.allocate().is_ok());
        assert!(allocator.allocate().is_ok());
        assert!(allocator.allocate().is_ok());

        // Fourth allocation should fail
        assert!(allocator.allocate().is_err());
    }

    #[test]
    fn test_invalid_range() {
        // Zero count
        assert!(PortAllocator::new(5700, 0).is_err());

        // Port 0 is not a usable service port
        assert!(PortAllocator::new(0, 10).is_err());

        // Overflow
        assert!(PortAllocator::new(65500, 100).is_err());
    }

    #[test]
    fn test_range_reaching_max_port() {
        // start + (count - 1) == 65535 must be accepted even though
        // start + count would overflow u16.
        let allocator = PortAllocator::new(65535, 1).unwrap();
        assert_eq!(allocator.range(), (65535, 65535));
        assert_eq!(allocator.available_count(), 1);

        // A large range ending exactly at 65535 also works and wraps correctly.
        let allocator = PortAllocator::new(65000, 536).unwrap();
        assert_eq!(allocator.range(), (65000, 65535));
        assert_eq!(allocator.available_count(), 536);
    }

    #[test]
    fn test_allocate_skips_port_in_use() {
        // Prove the allocator never hands out a port another process holds.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let busy = listener.local_addr().unwrap().port();
        if busy == u16::MAX {
            return; // No room for a second port above the busy one.
        }

        let mut allocator = PortAllocator::new(busy, 2).unwrap();
        assert_eq!(allocator.allocate().unwrap(), busy + 1);
        assert_eq!(allocator.available_count(), 0);
        assert!(allocator.allocate().is_err());
    }
}
