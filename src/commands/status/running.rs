//! # Running Status
//!
//! This module handles the display of system running status for foc-devnet services.
//!
//! It provides functionality to:
//! - Check Docker container status
//! - Display service uptime
//! - Show port accessibility
//! - Indicate overall system health

use tracing::info;

use crate::docker::core::image_exists;
use crate::docker::status::{
    get_container_ports, get_container_uptime, get_running_foc_containers,
};
use crate::run_id::load_current_run_id;

/// Print running status of the system in tabular format.
///
/// This function displays the status of all expected foc-devnet services,
/// including Docker containers, their uptime, and port accessibility.
/// If a run ID exists, it shows the actual container names with run ID prefix.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_devnet::commands::status::running_status::print_running_status;
///
/// print_running_status().expect("Failed to print running status");
/// ```
///
/// # Errors
///
/// Returns an error if Docker commands fail.
pub fn print_running_status() -> Result<(), Box<dyn std::error::Error>> {
    // Try to get current run ID
    let run_id = load_current_run_id().ok();

    // Check for running Docker containers
    let containers = get_running_foc_containers()?;

    let expected_containers = if let Some(ref id) = run_id {
        vec![
            ("Lotus Daemon", format!("foc-{}-lotus", id)),
            ("Lotus Miner", format!("foc-{}-lotus-miner", id)),
            ("YugabyteDB", format!("foc-{}-yugabyte", id)),
            ("Builder", format!("foc-{}-builder", id)),
        ]
    } else {
        vec![
            (
                "Lotus Daemon",
                crate::constants::LOTUS_CONTAINER.to_string(),
            ),
            (
                "Lotus Miner",
                crate::constants::LOTUS_MINER_CONTAINER.to_string(),
            ),
            ("Curio", crate::constants::CURIO_CONTAINER.to_string()),
            (
                "YugabyteDB",
                crate::constants::YUGABYTE_CONTAINER.to_string(),
            ),
            ("Builder", crate::constants::BUILDER_CONTAINER.to_string()),
        ]
    };

    if let Some(ref id) = run_id {
        info!("Run ID: {}", id);
    }

    for (service_name, container_name) in &expected_containers {
        let is_running = containers.contains(container_name);
        let image_name = extract_base_image_name(container_name);
        let image_available = image_exists(&image_name).unwrap_or(false);

        if is_running {
            let uptime = get_container_uptime(container_name)?;
            let ports = format_container_ports(container_name)?;
            info!(
                "{}: Running Container | Uptime: {} | Ports: {}",
                service_name, uptime, ports
            );
        } else if !image_available {
            info!("{}: Running Container Unavailable", service_name);
        } else {
            info!("{}: Stopped", service_name);
        }
    }

    // Check for Curio instances if run ID exists
    if let Some(ref id) = run_id {
        for sp_idx in 1..=5 {
            let curio_container = format!("foc-{}-curio-{}", id, sp_idx);
            if containers.contains(&curio_container) {
                let uptime = get_container_uptime(&curio_container)?;
                let ports = format_container_ports(&curio_container)?;
                info!(
                    "Curio SP-{}: Running | Uptime: {} | Ports: {}",
                    sp_idx, uptime, ports
                );
            }
        }
    }

    Ok(())
}

/// Extract base image name from container name.
///
/// Handles both run-id prefixed names (foc-<run_id>-<service>) and
/// simple names (foc-<service>).
fn extract_base_image_name(container_name: &str) -> String {
    if !container_name.starts_with("foc-") {
        return container_name.to_string();
    }

    let parts: Vec<&str> = container_name.split('-').collect();

    // foc-<run_id>-<service> or foc-<run_id>-<service>-<number>
    if parts.len() >= 3 {
        // Extract everything after the run_id part
        // For foc-26jan02-1058_TizzyTike-lotus -> LOTUS_CONTAINER
        // For foc-26jan02-1058_TizzyTike-curio-1 -> CURIO_CONTAINER
        let service_parts = &parts[2..];

        // If it ends with a number (like curio-1), remove it
        let ends_with_number = service_parts
            .last()
            .and_then(|s| s.parse::<u32>().ok())
            .is_some();

        if service_parts.len() >= 2 && ends_with_number {
            format!("foc-{}", service_parts[..service_parts.len() - 1].join("-"))
        } else {
            format!("foc-{}", service_parts.join("-"))
        }
    } else {
        container_name.to_string()
    }
}

/// Format container ports for display.
///
/// Extracts just the host port numbers from docker port output,
/// filtering out IPv6 bindings and showing only unique ports.
fn format_container_ports(container_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let ports_output = get_container_ports(container_name)?;
    let output = String::from_utf8_lossy(&ports_output.stdout);

    // Parse lines like "1234/tcp -> 0.0.0.0:5701" and extract just "5701"
    let mut ports = Vec::new();
    for line in output.lines() {
        if let Some(arrow_pos) = line.find("->") {
            let binding = &line[arrow_pos + 2..].trim();
            // Only take IPv4 bindings (0.0.0.0)
            if binding.starts_with("0.0.0.0:") {
                if let Some(colon_pos) = binding.rfind(':') {
                    let port = &binding[colon_pos + 1..];
                    ports.push(port.to_string());
                }
            }
        }
    }

    if ports.is_empty() {
        Ok("N/A".to_string())
    } else {
        Ok(ports.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_running_status() {
        // This test verifies that the function doesn't panic
        // In a real environment with Docker, it would check actual containers
        let result = print_running_status();
        // We expect this to work even if Docker is not available
        // (it will just show empty results)
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_base_image_name() {
        assert_eq!(
            extract_base_image_name("foc-26jan02-1058_TizzyTike-lotus"),
            crate::constants::LOTUS_CONTAINER
        );
        assert_eq!(
            extract_base_image_name("foc-26jan02-1058_TizzyTike-lotus-miner"),
            crate::constants::LOTUS_MINER_CONTAINER
        );
        assert_eq!(
            extract_base_image_name("foc-26jan02-1058_TizzyTike-curio-1"),
            crate::constants::CURIO_CONTAINER
        );
        assert_eq!(
            extract_base_image_name(crate::constants::LOTUS_CONTAINER),
            crate::constants::LOTUS_CONTAINER
        );
    }
}
