//! Port allocation module for managing dynamic port assignment.
//!
//! This module provides a `PortAllocator` that manages a contiguous range of ports
//! for the devnet cluster. All components (Lotus, Lotus-Miner, Curio, Yugabyte)
//! dynamically allocate ports from this pool, ensuring no conflicts.

use std::collections::HashSet;
use std::error::Error;
use std::net::TcpListener;

/// Manages dynamic allocation of ports from a contiguous range.
///
/// The allocator tracks which ports have been assigned and ensures
/// that each allocation is unique and within the configured range.
#[derive(Debug)]
pub struct PortAllocator {
    /// Starting port of the range
    start: u16,

    /// Total number of ports in the range
    count: u16,

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
    /// - The range would overflow u16 (start + count > 65535)
    pub fn new(start: u16, count: u16) -> Result<Self, Box<dyn Error>> {
        if count == 0 {
            return Err("Port range count must be greater than 0".into());
        }

        // Check for overflow
        let _end = start
            .checked_add(count.saturating_sub(1))
            .ok_or("Port range exceeds maximum port number (65535)")?;

        Ok(Self {
            start,
            count,
            allocated: HashSet::new(),
            next: start,
        })
    }

    /// Allocate a single port from the range.
    ///
    /// This method finds the next available port in the range and marks it as allocated.
    ///
    /// # Returns
    ///
    /// The allocated port number, or an error if no ports are available.
    ///
    /// # Errors
    ///
    /// Returns an error if all ports in the range have been allocated.
    pub fn allocate(&mut self) -> Result<u16, Box<dyn Error>> {
        // Try to find an available port starting from `next`
        let end = self.start + self.count;

        for _ in 0..self.count {
            let port = self.next;

            // Advance next port (wrap around if needed)
            self.next += 1;
            if self.next >= end {
                self.next = self.start;
            }

            // Check if this port is already allocated
            if !self.allocated.contains(&port) {
                self.allocated.insert(port);
                return Ok(port);
            }
        }

        Err(format!(
            "No available ports in range {}-{}",
            self.start,
            self.start + self.count - 1
        )
        .into())
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
        if port < self.start || port >= self.start + self.count {
            return Err(format!(
                "Port {} is outside the range {}-{}",
                port,
                self.start,
                self.start + self.count - 1
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
        self.count as usize - self.allocated.len()
    }

    /// Get the port range (start, end).
    pub fn range(&self) -> (u16, u16) {
        (self.start, self.start + self.count - 1)
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

        for port in self.start..(self.start + self.count) {
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

/// Check if a port is available (not in use).
///
/// # Arguments
///
/// * `port` - The port number to check
///
/// # Returns
///
/// `true` if the port is available, `false` if it's in use.
fn is_port_available(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
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

    #[test]
    fn test_allocator_creation() {
        let allocator = PortAllocator::new(5700, 300).unwrap();
        assert_eq!(allocator.range(), (5700, 5999));
        assert_eq!(allocator.available_count(), 300);
        assert_eq!(allocator.allocated_count(), 0);
    }

    #[test]
    fn test_allocate_single() {
        let mut allocator = PortAllocator::new(5700, 10).unwrap();
        let port1 = allocator.allocate().unwrap();
        let port2 = allocator.allocate().unwrap();

        assert_eq!(port1, 5700);
        assert_eq!(port2, 5701);
        assert_eq!(allocator.allocated_count(), 2);
        assert_eq!(allocator.available_count(), 8);
    }

    #[test]
    fn test_allocate_multiple() {
        let mut allocator = PortAllocator::new(5700, 20).unwrap();
        let ports = allocator.allocate_multiple(5).unwrap();

        assert_eq!(ports.len(), 5);
        assert_eq!(ports, vec![5700, 5701, 5702, 5703, 5704]);
        assert_eq!(allocator.allocated_count(), 5);
    }

    #[test]
    fn test_allocate_exhaustion() {
        let mut allocator = PortAllocator::new(5700, 3).unwrap();

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

        // Overflow
        assert!(PortAllocator::new(65500, 100).is_err());
    }
}
