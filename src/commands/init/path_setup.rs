//! PATH setup utilities for foc-localnet initialization.
//!
//! This module handles the setup of PATH environment variables in shell
//! configuration files (.bashrc and .zshrc).

use dirs;
use std::env;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

use crate::paths::foc_localnet_bin;

/// Set up PATH variables in shell configuration files.
///
/// This function adds the foc-localnet bin directory to the PATH in both
/// .bashrc and .zshrc files if it's not already present. It checks if the
/// path is already in the current environment before modifying files.
///
/// # Returns
/// Returns `Ok(())` if PATH setup is successful, or an error if file operations fail.
pub fn setup_path_variables() -> Result<(), Box<dyn std::error::Error>> {
    let bin_path = foc_localnet_bin();
    let bin_path_str = bin_path.to_string_lossy().to_string();

    if is_path_in_env(&bin_path_str) {
        info!("PATH already includes: {}", bin_path_str);
        return Ok(());
    }

    info!("Setting up PATH variables...");

    if let Some(home) = dirs::home_dir() {
        let bashrc = home.join(".bashrc");
        if let Err(e) = add_path_to_shell_config(&bashrc, &bin_path_str) {
            warn!("Failed to update .bashrc: {}", e);
        } else {
            info!("Updated .bashrc");
        }

        let zshrc = home.join(".zshrc");
        if let Err(e) = add_path_to_shell_config(&zshrc, &bin_path_str) {
            warn!("Failed to update .zshrc: {}", e);
        } else {
            info!("Updated .zshrc");
        }
    }

    Ok(())
}

/// Check if the given path is already in the PATH environment variable.
///
/// # Arguments
/// * `bin_path` - The path to check for in PATH
///
/// # Returns
/// Returns `true` if the path is found in PATH, `false` otherwise.
fn is_path_in_env(bin_path: &str) -> bool {
    let current_path = env::var("PATH").unwrap_or_default();
    current_path.split(':').any(|p| p == bin_path)
}

/// Add the bin path to a shell configuration file if not already present.
///
/// This function appends an export statement to the shell config file
/// to add the foc-localnet bin directory to PATH. It includes a marker
/// comment to prevent duplicate additions.
///
/// # Arguments
/// * `config_path` - Path to the shell configuration file
/// * `bin_path` - The bin directory path to add to PATH
///
/// # Returns
/// Returns `Ok(())` if the path is added successfully, or an error if file operations fail.
fn add_path_to_shell_config(
    config_path: &Path,
    bin_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !config_path.exists() {
        return Ok(());
    }

    let mut content = fs::read_to_string(config_path)?;
    let marker = "# foc-localnet PATH addition";

    if content.contains(marker) {
        return Ok(());
    }

    content.push_str(&format!(
        "\n{} \nexport PATH=\"$PATH:{}\"\n",
        marker, bin_path
    ));
    fs::write(config_path, content)?;
    Ok(())
}
