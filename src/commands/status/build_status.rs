//! # Build Status
//!
//! This module handles the display of build status information for foc-localnet binaries.
//!
//! It provides functionality to:
//! - Check if expected binaries exist
//! - Display build timestamps
//! - Show relative time since build

use crate::paths::foc_localnet_bin;
use chrono::{DateTime, Utc};
use crossterm::style::Stylize;
use tabular::{Row, Table};

use super::utils::{format_time_ago, get_terminal_width};

/// Print build status of artifacts in tabular format.
///
/// This function displays the build status of all expected foc-localnet binaries,
/// including whether they exist, their file sizes, and when they were last built.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::build_status::print_build_status;
///
/// print_build_status().expect("Failed to print build status");
/// ```
///
/// # Errors
///
/// Returns an error if file system operations fail.
pub fn print_build_status() -> Result<(), Box<dyn std::error::Error>> {
    let width = get_terminal_width().min(120);
    let header_text = format!("{} {}", "🔨".yellow(), "Build Status");
    // Display width: 🔨 (2) + space (1) + "Build Status" (12) + space (1) = 16
    let header_display_width = 2 + 1 + 12 + 1;
    let padding_len = width.saturating_sub(header_display_width);
    let padding = "░".repeat(padding_len).dark_grey();
    println!("\n{}{}{}", header_text.bold().yellow(), " ", padding);
    let width = get_terminal_width().min(120);
    println!("{}", "─".repeat(width).yellow());

    let bin_dir = foc_localnet_bin();

    // Check for expected binaries
    let expected_binaries = vec!["lotus", "lotus-miner", "lotus-shed", "lotus-seed", "curio"];

    // Create tabular output
    let mut table = Table::new("{:<}  {:<}  {:<}  {:<}");
    table.add_row(
        Row::new()
            .with_ansi_cell("Binary".bold().dark_grey())
            .with_ansi_cell("Status".bold().dark_grey())
            .with_ansi_cell("Path".bold().dark_grey())
            .with_ansi_cell("Time of Build".bold().dark_grey()),
    );

    for binary in expected_binaries {
        let binary_path = bin_dir.join(binary);
        let status = if binary_path.exists() {
            "Built".green().to_string()
        } else {
            "Not built".red().to_string()
        };
        let location = if binary_path.exists() {
            binary_path.display().to_string()
        } else {
            format!("{}/{}", bin_dir.display(), binary)
        };

        let build_time = if binary_path.exists() {
            match std::fs::metadata(&binary_path) {
                Ok(metadata) => match metadata.modified() {
                    Ok(modified) => {
                        let datetime: DateTime<Utc> = modified.into();
                        let now = Utc::now();
                        let duration = now.signed_duration_since(datetime);

                        let ago_str = format_time_ago(duration);
                        format!("{} ({})", datetime.format("%Y-%m-%d %H:%M"), ago_str)
                    }
                    Err(_) => "Unknown".to_string(),
                },
                Err(_) => "Unknown".to_string(),
            }
        } else {
            "N/A".to_string()
        };

        table.add_row(
            Row::new()
                .with_cell(binary)
                .with_ansi_cell(&status)
                .with_ansi_cell(location.dim())
                .with_ansi_cell(build_time.dim()),
        );
    }

    print!("{}", table);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_build_status() {
        // This test mainly verifies that the function doesn't panic
        // In a real scenario, you'd mock the foc_localnet_bin function
        // or set up the environment properly
        let result = print_build_status();
        // We expect this to work even if no binaries exist
        assert!(result.is_ok());
    }
}
