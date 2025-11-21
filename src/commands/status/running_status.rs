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

use super::docker::{get_container_uptime, get_port_status, get_running_containers};
use super::utils;

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
    let width = utils::get_terminal_width().min(120);
    let header_text = format!("{} {}", "⚙️".green(), "System Status");
    // Display width: ⚙️ (2) + space (1) + "System Status" (13) + space (1) = 17
    let header_display_width = 2 + 1 + 13 + 1;
    let padding_len = width.saturating_sub(header_display_width);
    let padding = "░".repeat(padding_len).dark_grey();
    println!("\n{}{}{}", header_text.bold().green(), " ", padding);
    let width = utils::get_terminal_width().min(120);
    println!("{}", "─".repeat(width).green());

    // Check for running Docker containers
    let containers = get_running_containers()?;

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

        // Special handling for builder - show as "Compiling" if running
        let status = if is_running {
            "Running".green().to_string()
        } else {
            // Don't count builder as "not running" for all_running check
            if *container_name != "foc-builder" {
                all_running = false;
            }
            "Stopped".red().to_string()
        };

        // Get uptime if container is running
        let uptime = if is_running {
            get_container_uptime(container_name)?
        } else {
            "N/A".dark_grey().to_string()
        };

        // Get port status if container is running
        let port_status = if is_running {
            get_port_status(container_name)?
        } else {
            "N/A".dark_grey().to_string()
        };

        table.add_row(
            Row::new()
                .with_cell(*service_name)
                .with_ansi_cell(&status)
                .with_cell(*container_name)
                .with_ansi_cell(&uptime)
                .with_ansi_cell(&port_status),
        );
    }

    print!("{}", table);
    let width = utils::get_terminal_width().min(120);
    println!("{}", "─".repeat(width).green());

    if all_running {
        println!("{}", "All services are running!".green().bold());
    } else {
        println!("{}", "Some services are not running.".yellow());
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
