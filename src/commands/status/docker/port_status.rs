//! Docker port status utilities.
//!
//! This module provides utilities for checking port mappings and accessibility.

use crossterm::style::Stylize;
use std::collections::HashMap;
use std::process::Command;

// Constants
const PORT_CHECK_TIMEOUT_MS: u64 = 100;

/// Get port status for a container, showing which ports are exposed and accessible.
///
/// This function checks the expected ports for a container and verifies if they
/// are accessible on the host system.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::docker::get_port_status;
///
/// let port_status = get_port_status("foc-yugabyte").unwrap();
/// println!("Port status: {}", port_status);
/// ```
///
/// # Parameters
///
/// * `container_name` - The name of the Docker container
///
/// # Returns
///
/// A formatted string showing port status, with colors indicating accessibility.
pub fn get_port_status(container_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Define expected ports for each container
    let expected_ports = get_expected_ports(container_name);

    if expected_ports.is_empty() {
        return Ok("-".dark_grey().to_string());
    }

    // Get actual port mappings from Docker
    let port_mappings = get_container_port_mappings(container_name)?;

    // Build port status string
    let mut port_status_parts = Vec::new();

    for (port, description) in expected_ports {
        let status = if let Some(host_port) = port_mappings.get(&port) {
            // Check if port is accessible on host
            if is_port_accessible("127.0.0.1", *host_port) {
                format!("{}({})", port, description).green().to_string()
            } else {
                format!("{}({})", port, description).red().to_string()
            }
        } else {
            format!("{}({})", port, description).red().to_string()
        };
        port_status_parts.push(status);
    }

    Ok(port_status_parts.join(" "))
}

/// Get expected ports and descriptions for a container.
///
/// This function returns the standard ports that should be exposed for each
/// foc-localnet service container.
///
/// # Examples
///
/// ```rust
/// use foc_localnet::commands::status::docker::get_expected_ports;
///
/// let ports = get_expected_ports("foc-yugabyte");
/// assert_eq!(ports.len(), 7); // Yugabyte has 7 expected ports
/// ```
///
/// # Parameters
///
/// * `container_name` - The name of the Docker container
///
/// # Returns
///
/// A vector of (port, description) tuples for the expected ports.
pub fn get_expected_ports(container_name: &str) -> Vec<(u16, &'static str)> {
    match container_name {
        "foc-yugabyte" => vec![
            (5433, "YSQL"),
            (9042, "YCQL"),
            (7000, "M-RPC"),
            (9000, "M-UI"),
            (7100, "T-RPC"),
            (9100, "T-UI"),
            (15433, "Web"),
        ],
        "foc-lotus" => vec![(1234, "API"), (5678, "P2P")],
        "foc-lotus-miner" => vec![(2345, "API")],
        "foc-curio" => vec![(12300, "API")],
        _ => vec![],
    }
}

/// Get port mappings for a container (container_port -> host_port).
///
/// This function parses the output of `docker port` to determine how container
/// ports are mapped to host ports.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::docker::get_container_port_mappings;
///
/// let mappings = get_container_port_mappings("foc-lotus").unwrap();
/// if let Some(host_port) = mappings.get(&1234) {
///     println!("Lotus API is mapped to host port {}", host_port);
/// }
/// ```
///
/// # Parameters
///
/// * `container_name` - The name of the Docker container
///
/// # Returns
///
/// A HashMap mapping container ports to host ports.
pub fn get_container_port_mappings(
    container_name: &str,
) -> Result<HashMap<u16, u16>, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["port", container_name])
        .output()?;

    let mut mappings = HashMap::new();

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_port_mappings(&stdout, &mut mappings);
    }

    Ok(mappings)
}

/// Parse the output of `docker port` command into a port mappings HashMap.
///
/// # Parameters
///
/// * `output` - The stdout from `docker port` command
/// * `mappings` - The HashMap to populate with port mappings
fn parse_port_mappings(output: &str, mappings: &mut HashMap<u16, u16>) {
    // Format: "5433/tcp -> 0.0.0.0:5433" or "5433/tcp -> [::]:5433"
    for line in output.lines() {
        if let Some((container_port_str, host_part)) = line.split_once(" -> ") {
            // Extract container port
            if let Some(port_str) = container_port_str.split('/').next() {
                if let Ok(container_port) = port_str.parse::<u16>() {
                    // Extract host port
                    if let Some(host_port_str) = host_part.split(':').last() {
                        if let Ok(host_port) = host_port_str.trim().parse::<u16>() {
                            mappings.insert(container_port, host_port);
                        }
                    }
                }
            }
        }
    }
}

/// Check if a port is accessible on the host.
///
/// This function attempts to connect to a port on the specified host with a short timeout.
///
/// # Examples
///
/// ```rust
/// use foc_localnet::commands::status::docker::is_port_accessible;
///
/// let accessible = is_port_accessible("127.0.0.1", 5433);
/// if accessible {
///     println!("Port 5433 is accessible");
/// } else {
///     println!("Port 5433 is not accessible");
/// }
/// ```
///
/// # Parameters
///
/// * `host` - The host IP address or hostname
/// * `port` - The port number to check
///
/// # Returns
///
/// `true` if the port is accessible, `false` otherwise.
pub fn is_port_accessible(host: &str, port: u16) -> bool {
    use std::net::TcpStream;
    use std::time::Duration;

    // Try to connect to the port with a short timeout
    match TcpStream::connect_timeout(
        &format!("{}:{}", host, port).parse().unwrap(),
        Duration::from_millis(PORT_CHECK_TIMEOUT_MS),
    ) {
        Ok(_) => true,
        Err(_) => false,
    }
}
