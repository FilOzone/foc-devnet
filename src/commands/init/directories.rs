//! Directory creation utilities for foc-localnet initialization.
//!
//! This module handles the creation of all necessary directories required
//! for foc-localnet to function properly.

use crossterm::style::Stylize;
use std::fs;

use crate::paths::{
    foc_localnet_artifacts, foc_localnet_bin, foc_localnet_code, foc_localnet_docker_volumes,
    foc_localnet_genesis, foc_localnet_genesis_sectors, foc_localnet_home, foc_localnet_logs,
    foc_localnet_lotus_keys, foc_localnet_proof_parameters, foc_localnet_state, foc_localnet_tmp,
};

/// Create all necessary directories for foc-localnet.
///
/// This function creates the following directories if they don't already exist:
/// - Home directory (~/.foc-localnet)
/// - Logs directory
/// - Bin directory
/// - State directory
/// - Code directory
/// - Temporary directory
/// - Artifacts directory
/// - Docker images directory
/// - Docker volumes directory
///
/// # Returns
/// Returns `Ok(())` if all directories are created successfully, or an error if creation fails.
pub fn create_directories() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Creating necessary directories...".bold());

    let directories = vec![
        foc_localnet_home(),
        foc_localnet_logs(),
        foc_localnet_bin(),
        foc_localnet_state(),
        foc_localnet_code(),
        foc_localnet_tmp(),
        foc_localnet_artifacts(),
        foc_localnet_docker_volumes(),
        foc_localnet_proof_parameters(),
        foc_localnet_lotus_keys(),
        foc_localnet_genesis_sectors(),
        foc_localnet_genesis(),
    ];

    for dir in directories {
        if !dir.exists() {
            println!("  {} Creating directory: {:?}", "ℹ".cyan(), dir);
            fs::create_dir_all(&dir)?;
            println!("\r  {} Created: {}", "✓".green(), dir.display());
        } else {
            println!("  {} Exists: {}", "✓".green(), dir.display());
        }
    }

    Ok(())
}
