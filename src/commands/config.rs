//! Configuration management commands.
//!
//! This module provides commands for updating the foc-devnet configuration,
//! specifically for changing the source locations of Lotus and Curio components.

use crate::config::{Config, Location};
use crate::paths::foc_devnet_config;
use std::fs;
use tracing::info;

/// Configure the Lotus source location in the config file.
///
/// This function updates the `lotus` field in the configuration with the
/// provided source location string. The source can be in formats like:
/// - `gittag:v1.0.0` (uses default Lotus repository URL)
/// - `gitcommit:abc123` (uses default Lotus repository URL)
/// - `local:/path/to/lotus` (local directory)
/// - `gittag:https://github.com/user/lotus.git:v1.0.0` (custom URL)
pub fn config_lotus(source: String) -> Result<(), Box<dyn std::error::Error>> {
    update_config_location(
        "lotus",
        source,
        "https://github.com/filecoin-project/lotus.git",
    )
}

/// Configure the Curio source location in the config file.
///
/// This function updates the `curio` field in the configuration with the
/// provided source location string. The source can be in formats like:
/// - `gittag:v1.0.0` (uses default Curio repository URL)
/// - `gitcommit:abc123` (uses default Curio repository URL)
/// - `local:/path/to/curio` (local directory)
/// - `gittag:https://github.com/user/curio.git:v1.0.0` (custom URL)
pub fn config_curio(source: String) -> Result<(), Box<dyn std::error::Error>> {
    update_config_location(
        "curio",
        source,
        "https://github.com/filecoin-project/curio.git",
    )
}

/// Internal function to update a location field in the config.
///
/// # Arguments
/// * `field` - The field name ("lotus" or "curio")
/// * `source` - The source location string to parse
/// * `default_url` - The default repository URL for the component
fn update_config_location(
    field: &str,
    source: String,
    default_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load existing config
    let config_path = foc_devnet_config();
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
    let mut config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Parse the source location
    let location = Location::resolve_with_default(&source, default_url)
        .map_err(|e| format!("Invalid {} source format: {}", field, e))?;

    // Update the appropriate field
    match field {
        "lotus" => config.lotus = location,
        "curio" => config.curio = location,
        _ => return Err(format!("Unknown field: {}", field).into()),
    }

    // Write back the config
    let updated_content = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&config_path, updated_content)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    info!(
        "Successfully updated {} configuration to: {}",
        field, source
    );
    Ok(())
}
