//! # Build Status
//!
//! This module handles the display of build status information for foc-devnet binaries.
//!
//! It provides functionality to:
//! - Check if expected binaries exist
//! - Display build timestamps
//! - Show relative time since build

use crate::{constants::REQUIRED_BINARIES, paths::foc_devnet_bin};
use chrono::{DateTime, Utc};
use std::process::Command;
use tracing::info;

use super::utils::format_time_ago;

/// Print build status of artifacts in tabular format.
///
/// This function displays the build status of all expected foc-devnet binaries,
/// including whether they exist, their file sizes, and when they were last built.
///
/// Note: The list of expected binaries is shared with the startup binary check
/// to ensure consistency.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_devnet::commands::status::build_status::print_build_status;
///
/// print_build_status().expect("Failed to print build status");
/// ```
///
/// # Errors
///
/// Returns an error if file system operations fail.
pub fn print_build_status() -> Result<(), Box<dyn std::error::Error>> {
    let bin_dir = foc_devnet_bin();

    // Get expected binaries from shared source of truth
    let expected_binaries = REQUIRED_BINARIES;

    for binary in expected_binaries {
        let binary_path = bin_dir.join(binary);
        if binary_path.exists() {
            let metadata = std::fs::metadata(&binary_path)?;
            let modified: DateTime<Utc> = metadata.modified()?.into();
            let time_ago = format_time_ago(Utc::now() - modified);

            // Get version information
            let version = get_binary_version(&binary_path);

            info!(
                "Binary: {}: Ready | Built {} ({}) | Version: {}",
                binary,
                modified.format("%Y-%m-%d %H:%M"),
                time_ago,
                version
            );
        } else {
            info!("Binary \"{}\": Missing", binary);
        }
    }

    Ok(())
}

/// Get the version string for a binary by executing it with --version.
///
/// Returns the version string or "Unknown" if the command fails.
fn get_binary_version(binary_path: &std::path::Path) -> String {
    match Command::new(binary_path).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Take first line and clean it up
            stdout
                .lines()
                .next()
                .unwrap_or("Unknown")
                .trim()
                .to_string()
        }
        _ => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_build_status() {
        // This test mainly verifies that the function doesn't panic
        // In a real scenario, you'd mock the foc_devnet_bin function
        // or set up the environment properly
        let result = print_build_status();
        // We expect this to work even if no binaries exist
        assert!(result.is_ok());
    }
}
