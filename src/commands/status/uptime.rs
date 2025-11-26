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
use std::process::Command;

use super::docker::{get_running_containers, get_system_start_time};
use super::utils::{format_duration, get_terminal_width};

/// Get the current lotus chain block height.
///
/// This function queries the lotus node to get the current block height of the chain.
/// Returns `None` if the lotus container is not running or if the command fails.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::uptime::get_lotus_block_height;
///
/// if let Some(height) = get_lotus_block_height() {
///     println!("Current block height: {}", height);
/// }
/// ```
fn get_lotus_block_height() -> Option<u64> {
    let output = Command::new("docker")
        .args([
            "exec",
            "foc-lotus",
            "/usr/local/bin/lotus-bins/lotus",
            "chain",
            "list",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?;

    // Parse the block height from the first line (format: "HEIGHT: (timestamp) [ ... ]")
    let height_str = first_line.split(':').next()?.trim();
    height_str.parse::<u64>().ok()
}

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

        // Try to get lotus block height if chain is running
        if let Some(block_height) = get_lotus_block_height() {
            println!(
                "{} {}",
                "Chain height (lotus):".green(),
                block_height.to_string().green().bold()
            );
        }
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

    #[test]
    fn test_get_lotus_block_height() {
        // This test verifies that the function doesn't panic
        let _height = get_lotus_block_height();
        // We don't assert anything as the result depends on whether lotus is running
    }
}
