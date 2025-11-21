//! # System Uptime
//!
//! This module handles the display of system uptime information for foc-localnet.
//!
//! It provides functionality to:
//! - Calculate total system uptime based on container start times
//! - Display formatted uptime strings
//! - Handle cases where the system is not running

use chrono::Utc;
use crossterm::style::Stylize;

use super::docker::{get_running_containers, get_system_start_time};
use super::utils::{format_duration, get_terminal_width};

/// Print uptime information if system is running.
///
/// This function displays the total uptime of the foc-localnet system by finding
/// the oldest running container and calculating how long the system has been running.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::uptime::print_uptime;
///
/// print_uptime().expect("Failed to print uptime");
/// ```
///
/// # Errors
///
/// Returns an error if Docker commands fail.
pub fn print_uptime() -> Result<(), Box<dyn std::error::Error>> {
    let width = get_terminal_width().min(120);
    let header_text = format!("{} {}", "⏱️".magenta(), "System Uptime");
    // Display width: ⏱️ (2) + space (1) + "System Uptime" (13) + space (1) = 17
    let header_display_width = 2 + 1 + 13 + 1;
    let padding_len = width.saturating_sub(header_display_width);
    let padding = "░".repeat(padding_len).dark_grey();
    println!("\n{}{}{}", header_text.bold().magenta(), " ", padding);
    let width = get_terminal_width().min(120);
    println!("{}", "─".repeat(width).magenta());

    let containers = get_running_containers()?;

    if containers.is_empty() {
        println!("{}", "System is not running".red());
        return Ok(());
    }

    // Get the oldest container start time as system start time
    if let Some(start_time) = get_system_start_time()? {
        let now = Utc::now();
        let uptime = now.signed_duration_since(start_time);

        let total_seconds = uptime.num_seconds();
        let uptime_str = format_duration(total_seconds as i64);

        println!("{} {}", "System uptime:".green(), uptime_str.green().bold());
    } else {
        println!("{}", "Unable to determine uptime".yellow());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_uptime() {
        // This test verifies that the function doesn't panic
        let result = print_uptime();
        // We expect this to work even if no containers are running
        assert!(result.is_ok());
    }
}
