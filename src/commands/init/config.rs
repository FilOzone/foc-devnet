//! Configuration generation utilities for foc-localnet initialization.
//!
//! This module handles the generation of default configuration files
//! and application of location overrides.

use crossterm::style::Stylize;
use std::fs;

use crate::config::{Config, Location};
use crate::paths::foc_localnet_config;

/// Generate default configuration file if it doesn't exist.
///
/// This function creates a default `config.toml` file in the foc-localnet
/// configuration directory. If the file already exists and `force` is false,
/// it skips generation. If `force` is true, it overwrites the existing file.
///
/// Location overrides can be provided for Curio, Lotus, Filecoin Services, and Yugabyte URL.
///
/// # Arguments
/// * `curio_location` - Optional override for Curio repository location
/// * `lotus_location` - Optional override for Lotus repository location
/// * `filecoin_services_location` - Optional override for Filecoin Services repository location
/// * `yugabyte_url` - Optional override for Yugabyte download URL
/// * `force` - Whether to force regeneration of config file
///
/// # Returns
/// Returns `Ok(())` on successful config generation, or an error if generation fails.
pub fn generate_default_config(
    curio_location: Option<String>,
    lotus_location: Option<String>,
    filecoin_services_location: Option<String>,
    yugabyte_url: Option<String>,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = foc_localnet_config();

    if config_path.exists() && !force {
        println!(
            "  {} Config file already exists: {}",
            "✓".green(),
            config_path.display()
        );
        return Ok(());
    }

    if config_path.exists() && force {
        println!(
            "  {} Removing existing config file due to --force",
            "⚠".yellow()
        );
        std::fs::remove_file(&config_path)?;
    }

    println!(
        "  {} Generating default config: {:?}",
        "ℹ".cyan(),
        config_path
    );

    // Start with default config
    let mut config = Config::default();

    // Apply location overrides
    apply_location_override(
        &mut config.lotus,
        lotus_location,
        "https://github.com/filecoin-project/lotus.git",
    )?;
    apply_location_override(
        &mut config.curio,
        curio_location,
        "https://github.com/filecoin-project/curio.git",
    )?;
    apply_location_override(
        &mut config.filecoin_services,
        filecoin_services_location,
        "https://github.com/FilOzone/filecoin-services.git",
    )?;

    // Override yugabyte URL if provided
    if let Some(url) = yugabyte_url {
        config.yugabyte_download_url = url;
    }

    let default_config = toml::to_string(&config)
        .map_err(|e| format!("Failed to serialize default config: {}", e))?;

    fs::write(&config_path, default_config)?;
    println!(
        "  {} Created default config: {}",
        "✓".green(),
        config_path.display()
    );

    Ok(())
}

/// Apply a location override to a Location field if the override string is provided.
///
/// This function parses the override string and updates the location accordingly.
/// If no override is provided, the location remains unchanged.
///
/// # Arguments
/// * `location` - The location field to potentially update
/// * `override_str` - The override string to parse (optional)
/// * `default_url` - Default URL to use if location is LocalSource
///
/// # Returns
/// Returns `Ok(())` if override is applied successfully, or an error if parsing fails.
pub fn apply_location_override(
    location: &mut Location,
    override_str: Option<String>,
    default_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(loc_str) = override_str {
        let url = match *location {
            Location::GitTag { ref url, .. } => url.clone(),
            Location::GitCommit { ref url, .. } => url.clone(),
            Location::GitBranch { ref url, .. } => url.clone(),
            Location::LocalSource { .. } => default_url.to_string(),
        };
        *location = Location::parse_with_default(&loc_str, &url)
            .map_err(|e| format!("Invalid location: {}", e))?;
    }
    Ok(())
}
