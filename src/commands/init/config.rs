//! Configuration generation utilities for foc-devnet initialization.
//!
//! This module handles the generation of default configuration files
//! and application of location overrides.

use std::fs;
use tracing::info;

use crate::config::{Config, Location};
use crate::paths::foc_devnet_config;

/// Generate default configuration file if it doesn't exist.
///
/// If a config.toml already exists (e.g. preserved across `clean`), it is
/// reused as-is with any CLI location overrides applied on top. Otherwise a
/// fresh default config is created.
///
/// Location overrides can be provided for Curio, Lotus, and Filecoin Services.
///
/// # Arguments
/// * `curio_location` - Optional override for Curio repository location
/// * `lotus_location` - Optional override for Lotus repository location
/// * `filecoin_services_location` - Optional override for Filecoin Services repository location
/// * `pdp_location` - Optional override for PDP repository location
///
/// # Returns
/// Returns `Ok(())` on successful config generation, or an error if generation fails.
pub fn generate_default_config(
    curio_location: Option<String>,
    lotus_location: Option<String>,
    filecoin_services_location: Option<String>,
    pdp_location: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = foc_devnet_config();

    if config_path.exists() {
        info!("Reusing existing config: {}", config_path.display());
        // Apply any CLI location overrides to the existing config
        if curio_location.is_some()
            || lotus_location.is_some()
            || filecoin_services_location.is_some()
            || pdp_location.is_some()
        {
            let content = fs::read_to_string(&config_path)?;
            let mut config: Config = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse existing config: {}", e))?;
            apply_overrides(
                &mut config,
                curio_location,
                lotus_location,
                filecoin_services_location,
                pdp_location,
            )?;
            let updated = toml::to_string(&config)
                .map_err(|e| format!("Failed to serialize config: {}", e))?;
            fs::write(&config_path, updated)?;
            info!("Applied location overrides to existing config");
        }
        return Ok(());
    }

    info!("Generating default config: {:?}", config_path);

    // Start with default config
    let mut config = Config::default();

    // Apply location overrides
    apply_overrides(
        &mut config,
        curio_location,
        lotus_location,
        filecoin_services_location,
        pdp_location,
    )?;

    let default_config = toml::to_string(&config)
        .map_err(|e| format!("Failed to serialize default config: {}", e))?;

    fs::write(&config_path, default_config)?;
    info!("Created default config: {}", config_path.display());

    Ok(())
}

/// Apply location overrides to a config.
fn apply_overrides(
    config: &mut Config,
    curio_location: Option<String>,
    lotus_location: Option<String>,
    filecoin_services_location: Option<String>,
    pdp_location: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
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
    if let Some(pdp_location) = pdp_location {
        let mut pdp = config.pdp.clone().unwrap_or_else(|| Location::GitBranch {
            url: "https://github.com/FilOzone/pdp.git".to_string(),
            branch: "main".to_string(),
        });
        apply_location_override(
            &mut pdp,
            Some(pdp_location),
            "https://github.com/FilOzone/pdp.git",
        )?;
        config.pdp = Some(pdp);
    }

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
        *location = Location::resolve_with_default(&loc_str, &url)
            .map_err(|e| format!("Invalid location: {}", e))?;
    }
    Ok(())
}
