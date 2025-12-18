//! # Running Status
//!
//! This module handles the display of system running status for foc-localnet services.
//!
//! It provides functionality to:
//! - Check Docker container status
//! - Display service uptime
//! - Show port accessibility
//! - Indicate overall system health

use crossterm::style::Stylize;
use tabular::{Row, Table};
use tracing::{info, warn};

use crate::docker::core::image_exists;
use crate::docker::status::{
    get_container_ports, get_container_uptime, get_running_foc_containers,
};

/// Print running status of the system in tabular format.
///
/// This function displays the status of all expected foc-localnet services,
/// including Docker containers, their uptime, and port accessibility.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::running_status::print_running_status;
///
/// print_running_status().expect("Failed to print running status");
/// ```
///
/// # Errors
///
/// Returns an error if Docker commands fail.
pub fn print_running_status() -> Result<(), Box<dyn std::error::Error>> {
    info!("System Status");

    // Check for running Docker containers
    let containers = get_running_foc_containers()?;

    let expected_containers = vec![
        ("Lotus Daemon", "foc-lotus"),
        ("Lotus Miner", "foc-lotus-miner"),
        ("Curio", "foc-curio"),
        ("YugabyteDB", "foc-yugabyte"),
        ("Builder", "foc-builder"),
    ];

    // Create tabular output
    let mut table = Table::new("{:<}  {:<}  {:<}  {:<}  {:<}");
    table.add_row(
        Row::new()
            .with_ansi_cell("Service".bold().dark_grey())
            .with_ansi_cell("Status".bold().dark_grey())
            .with_ansi_cell("Container".bold().dark_grey())
            .with_ansi_cell("Uptime".bold().dark_grey())
            .with_ansi_cell("Ports".bold().dark_grey()),
    );

    let mut all_running = true;
    for (service_name, container_name) in &expected_containers {
        let is_running = containers.contains(&container_name.to_string());
        let image_available = image_exists(container_name).unwrap_or(false);

        // Determine status based on image availability and running state
        let status = if !image_available {
            "Unavailable".to_string()
        } else if is_running {
            "Running".to_string()
        } else {
            // Don't count builder as "not running" for all_running check
            if *container_name != "foc-builder" {
                all_running = false;
            }
            "Stopped".to_string()
        };

        // Get uptime if container is running
        let uptime = if is_running {
            get_container_uptime(container_name)?
        } else {
            "N/A".to_string()
        };

        // Get port status if container is running
        let port_status = if is_running {
            let ports_output = get_container_ports(container_name)?;
            String::from_utf8_lossy(&ports_output.stdout)
                .trim()
                .to_string()
        } else {
            "N/A".to_string()
        };

        table.add_row(
            Row::new()
                .with_cell(*service_name)
                .with_cell(status)
                .with_cell(*container_name)
                .with_cell(uptime)
                .with_cell(port_status),
        );
    }

    for line in table.to_string().lines() {
        info!("{}", line);
    }

    if all_running {
        info!("All services are running!");
    } else {
        warn!("Some services are not running.");
    }

    Ok(())
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
}
