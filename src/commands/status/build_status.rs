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
use tracing::info;

use super::utils::format_time_ago;

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
    let bin_dir = foc_localnet_bin();

    // Check for expected binaries
    let expected_binaries = vec!["lotus", "lotus-miner", "lotus-shed", "lotus-seed", "curio"];

    for binary in expected_binaries {
        let binary_path = bin_dir.join(binary);
        if binary_path.exists() {
            let metadata = std::fs::metadata(&binary_path)?;
            let modified: DateTime<Utc> = metadata.modified()?.into();
            let time_ago = format_time_ago(Utc::now() - modified);
            info!(
                "Binary \"{}\": build on {} ({})",
                binary,
                modified.format("%Y-%m-%d %H:%M"),
                time_ago
            );
        } else {
            info!("Binary \"{}\": Missing", binary);
        }
    }

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
