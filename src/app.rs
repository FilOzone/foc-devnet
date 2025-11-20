use std::fs;
use std::path::Path;
use std::env;
use dirs;
use tracing::info;

use crate::paths::{foc_localnet_bin, foc_localnet_config};

/// Check if the given path is already in the PATH environment variable.
fn is_path_in_env(bin_path: &str) -> bool {
    let current_path = env::var("PATH").unwrap_or_default();
    current_path.split(':').any(|p| p == bin_path)
}

/// Add the bin path to a shell configuration file if not already present.
fn add_path_to_shell_config(config_path: &Path, bin_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !config_path.exists() {
        return Ok(());
    }

    let mut content = fs::read_to_string(config_path)?;
    let marker = "# foc-localnet PATH addition";

    if content.contains(marker) {
        return Ok(());
    }

    content.push_str(&format!("\n{} \nexport PATH=\"$PATH:{}\"\n", marker, bin_path));
    fs::write(config_path, content)?;
    Ok(())
}

/// Initialize the application environment.
///
/// This function sets up the necessary directories and configuration files
/// for the foc-localnet application. It ensures the application directory
/// exists and installs a default configuration if none is present.
pub fn initialize_app() -> Result<(), Box<dyn std::error::Error>> {
    // Check and create application directory
    let config_file = foc_localnet_config();
    if !config_file.parent().unwrap().exists() {
        info!("Creating application directory: {:?}", config_file.parent().unwrap());
        fs::create_dir_all(config_file.parent().unwrap())?;
    }

    if !config_file.exists() {
        info!("Setting up default config: {:?}", config_file);
        let default_config = toml::to_string(&crate::config::Config::default()).unwrap();
        fs::write(&config_file, default_config)?;
    }

    // Check if PATH includes foc_localnet_bin(), if not, add it.
    let bin_path = foc_localnet_bin();
    let bin_path_str = bin_path.to_string_lossy().to_string();

    if !is_path_in_env(&bin_path_str) {
        if let Some(home) = dirs::home_dir() {
            let bashrc = home.join(".bashrc");
            if let Err(e) = add_path_to_shell_config(&bashrc, &bin_path_str) {
                tracing::warn!("Failed to update .bashrc: {}", e);
            }

            let zshrc = home.join(".zshrc");
            if let Err(e) = add_path_to_shell_config(&zshrc, &bin_path_str) {
                tracing::warn!("Failed to update .zshrc: {}", e);
            }
        }
    }

    Ok(())
}

/// Initialize tracing/logging for the application.
pub fn init_tracing() {
    tracing_subscriber::fmt::init();
}
