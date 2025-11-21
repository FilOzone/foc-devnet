//! # Status Module
//!
//! This module provides comprehensive status reporting for the FOC LocalNet system.
//!
//! The status command displays information about:
//! - Code versions and git status for Lotus and Curio repositories
//! - Build status of system binaries
//! - Running status of Docker containers and services
//! - System uptime information
//! - Disk usage across various directories
//!
//! ## Usage
//!
//! ```rust
//! use foc_localnet::commands::status;
//!
//! // Display full system status
//! status::status()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Architecture
//!
//! The status module is organized into several submodules:
//! - `code_version`: Handles repository version and git information display
//! - `build_status`: Manages binary build status reporting
//! - `running_status`: Reports Docker container and service status
//! - `uptime`: Calculates and displays system uptime
//! - `disk_usage`: Provides disk usage statistics
//! - `git`: Git repository utilities
//! - `docker`: Docker container management utilities
//! - `utils`: Common formatting and utility functions

pub mod build_status;
pub mod code_version;
pub mod disk_usage;
pub mod docker;
pub mod git;
pub mod running_status;
pub mod uptime;
pub mod utils;

use crossterm::style::Stylize;

/// Execute the status command.
///
/// This function displays a pretty-printed status of the foc-localnet system,
/// including code version, build status, running status, and uptime information.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status;
///
/// // Display the current system status
/// status::status().expect("Failed to display status");
/// ```
///
/// # Errors
///
/// Returns an error if any of the status gathering operations fail, such as:
/// - Unable to read configuration files
/// - Git repository access issues
/// - Docker command execution failures
/// - File system access problems
pub fn status() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "🚀 FOC LocalNet Status".bold().cyan().underlined());
    println!("{}", "═".repeat(120).cyan());

    // Code version information
    code_version::print_code_version()?;

    // Artifacts build status
    build_status::print_build_status()?;

    // System running status
    running_status::print_running_status()?;

    // Uptime information (if running)
    uptime::print_uptime()?;

    // Disk usage information
    disk_usage::print_disk_usage()?;

    Ok(())
}
