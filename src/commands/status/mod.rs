//! # Status Module
//!
//! This module provides comprehensive status reporting for the FOC LocalNet system.
//!
//! The status command displays information about:
//! - Code versions and git status for repositories (Lotus, Curio, Filecoin-Services, Synapse-SDK)
//! - Build status of system binaries
//! - Proof parameters availability and validation
//! - Running status of Docker containers and services
//! - System uptime information
//! - Running system details (block height, ports, file locations) when system is active
//! - Generated keys and their file locations
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
//! - `proof_params`: Checks proof parameters availability and validation
//! - `running_status`: Reports Docker container and service status
//! - `uptime`: Calculates and displays system uptime
//! - `running_system_info`: Shows detailed info when system is running (ports, block height, files)
//! - `keys`: Displays generated keys and their file locations
//! - `git`: Git repository utilities
//! - `utils`: Common formatting and utility functions

pub mod build_status;
pub mod code_version;
pub mod git;
pub mod keys;
pub mod proof_params;
pub mod running;
pub mod running_system_info;
pub mod uptime;
pub mod utils;

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
    // Code version information
    code_version::print_code_version()?;

    // Artifacts build status
    build_status::print_build_status()?;

    // Proof parameters status
    proof_params::print_proof_params_status()?;

    // System running status
    running::print_running_status()?;

    // Uptime information (if running)
    uptime::print_uptime()?;

    // Running system information (ports, block height, files) - only if system is running
    running_system_info::print_running_system_info()?;

    // Keys information
    keys::print_keys_status()?;

    Ok(())
}
