//! # Docker Utilities
//!
//! This module provides utilities for interacting with Docker containers
//! and managing container-related status information.
//!
//! It includes functions for:
//! - Listing running containers
//! - Getting container uptime
//! - Checking port mappings and accessibility
//! - Parsing Docker command output

use chrono::{DateTime, Utc};
use crossterm::style::Stylize;
use std::collections::HashMap;
use std::process::Command;

/// Check if a Docker image exists locally in the Docker daemon.
///
/// This function checks if the Docker image exists in the local Docker daemon
/// using `docker images`.
///
/// # Arguments
/// * `image_tag` - The tag of the Docker image to check (e.g., "foc-builder")
///
/// # Returns
/// Returns `true` if the image exists in the local Docker daemon, `false` otherwise.
pub fn image_exists(image_tag: &str) -> bool {
    let output = Command::new("docker")
        .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().any(|line| line.starts_with(&format!("{}:", image_tag)))
        }
        _ => false,
    }
}

/// Get list of running Docker containers with foc- prefix.
///
/// This function queries Docker for all running containers whose names start with "foc-".
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::docker::get_running_containers;
///
/// let containers = get_running_containers().unwrap();
/// for container in containers {
///     println!("Running: {}", container);
/// }
/// ```
///
/// # Errors
///
/// Returns an error if the Docker command fails to execute.
pub fn get_running_containers() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=foc-", "--format", "{{.Names}}"])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let containers: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(containers)
}

/// Get uptime for a running container.
///
/// This function retrieves the container's start time from Docker and calculates
/// how long it has been running.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::docker::get_container_uptime;
///
/// let uptime = get_container_uptime("foc-lotus").unwrap();
/// println!("Container uptime: {}", uptime);
/// ```
///
/// # Parameters
///
/// * `container_name` - The name of the Docker container
///
/// # Errors
///
/// Returns an error if the Docker command fails or if the timestamp cannot be parsed.
pub fn get_container_uptime(container_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args([
            "inspect",
            container_name,
            "--format",
            "{{.State.StartedAt}}",
        ])
        .output()?;

    if !output.status.success() {
        return Ok("Unknown".dark_grey().to_string());
    }

    let started_at_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse the datetime string from Docker
    if let Ok(started_at) = DateTime::parse_from_rfc3339(&started_at_str) {
        let started_at_utc: DateTime<Utc> = started_at.with_timezone(&Utc);
        let now = Utc::now();
        let duration = now.signed_duration_since(started_at_utc);

        let days = duration.num_days();
        let hours = duration.num_hours() % 24;
        let minutes = duration.num_minutes() % 60;
        let seconds = duration.num_seconds() % 60;

        let uptime_str = if days > 0 {
            format!("{}d {}h", days, hours)
        } else if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        };

        Ok(uptime_str.green().to_string())
    } else {
        Ok("Unknown".dark_grey().to_string())
    }
}

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
        // Format: "5433/tcp -> 0.0.0.0:5433" or "5433/tcp -> [::]:5433"
        for line in stdout.lines() {
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

    Ok(mappings)
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
        Duration::from_millis(100),
    ) {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Get the system start time (oldest container start time).
///
/// This function finds the earliest start time among all running foc-localnet containers
/// to determine when the system was started.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::docker::get_system_start_time;
///
/// if let Some(start_time) = get_system_start_time().unwrap() {
///     println!("System started at: {}", start_time);
/// } else {
///     println!("Could not determine system start time");
/// }
/// ```
///
/// # Returns
///
/// An `Option<DateTime<Utc>>` representing the system start time, or `None` if it cannot be determined.
pub fn get_system_start_time() -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=foc-", "--format", "{{.RunningFor}}"])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut earliest_start: Option<DateTime<Utc>> = None;

    for line in stdout.lines() {
        if let Some(start_time) = parse_docker_running_for(line.trim()) {
            if earliest_start.is_none() || start_time < earliest_start.unwrap() {
                earliest_start = Some(start_time);
            }
        }
    }

    Ok(earliest_start)
}

/// Parse Docker "Running for" time string into DateTime.
///
/// This function parses the human-readable time strings that Docker provides
/// (like "2 hours", "3 minutes ago") and converts them to DateTime objects.
///
/// # Examples
///
/// ```rust
/// use foc_localnet::commands::status::docker::parse_docker_running_for;
///
/// let datetime = parse_docker_running_for("2 hours");
/// assert!(datetime.is_some());
///
/// let datetime = parse_docker_running_for("30 minutes ago");
/// assert!(datetime.is_some());
/// ```
///
/// # Parameters
///
/// * `running_for` - The Docker "Running for" string to parse
///
/// # Returns
///
/// An `Option<DateTime<Utc>>` representing the parsed time, or `None` if parsing fails.
pub fn parse_docker_running_for(running_for: &str) -> Option<DateTime<Utc>> {
    // Docker formats: "2 hours", "3 minutes ago", "About an hour ago", etc.
    // This is a simplified parser - in a real implementation you might want more robust parsing
    let now = Utc::now();

    if running_for.contains("second") {
        let seconds: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::seconds(seconds))
    } else if running_for.contains("minute") {
        let minutes: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::minutes(minutes))
    } else if running_for.contains("hour") {
        let hours: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::hours(hours))
    } else if running_for.contains("day") {
        let days: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::days(days))
    } else if running_for.contains("week") {
        let weeks: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        Some(now - chrono::Duration::weeks(weeks))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_expected_ports() {
        let yugabyte_ports = get_expected_ports("foc-yugabyte");
        assert_eq!(yugabyte_ports.len(), 7);
        assert_eq!(yugabyte_ports[0], (5433, "YSQL"));

        let lotus_ports = get_expected_ports("foc-lotus");
        assert_eq!(lotus_ports.len(), 2);
        assert_eq!(lotus_ports[0], (1234, "API"));

        let unknown_ports = get_expected_ports("unknown");
        assert_eq!(unknown_ports.len(), 0);
    }

    #[test]
    fn test_parse_docker_running_for() {
        let now = Utc::now();

        let result = parse_docker_running_for("5 seconds");
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert!(parsed <= now);

        let result = parse_docker_running_for("2 hours");
        assert!(result.is_some());

        let result = parse_docker_running_for("invalid");
        assert!(result.is_none());
    }
}
