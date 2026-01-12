//! FOC deployment helper functions.
//!
//! This module contains helper functions for FOC contract deployment,
//! including repository path resolution and deployment checks.

use crate::config::{Config, Location};
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use crate::paths::{foc_devnet_config, foc_devnet_filecoin_services_repo};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Get the filecoin-services repository path based on configuration
///
/// # Returns
/// The path to the filecoin-services repository
pub fn get_filecoin_services_repo_path() -> Result<PathBuf, Box<dyn Error>> {
    // Load configuration
    let config_path = foc_devnet_config();
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Determine the repository path based on location
    let repo_path = match &config.filecoin_services {
        Location::LocalSource { dir } => {
            // For LocalSource, use the configured directory directly
            PathBuf::from(dir)
        }
        _ => {
            // For Git-based locations, use the foc-devnet directory
            foc_devnet_filecoin_services_repo()
        }
    };

    Ok(repo_path)
}

/// Check if Lotus is running and accessible
///
/// # Arguments
/// * `context` - The step context containing run ID and other state
///
/// # Returns
/// Ok(()) if Lotus is running, error otherwise
pub fn check_lotus_running(
    context: &super::super::step::SetupContext,
) -> Result<(), Box<dyn Error>> {
    let run_id = context.run_id();
    let lotus_name = lotus_container_name(run_id);

    if !container_is_running(&lotus_name)? {
        return Err(format!(
            "Lotus container '{}' is not running. FOC deployment requires Lotus to be running with FEVM enabled.",
            lotus_name
        ).into());
    }
    Ok(())
}

/// Check if required addresses are available in context
///
/// # Arguments
/// * `context` - The step context containing addresses
///
/// # Returns
/// Tuple of (foc_deployer, foc_deployer_eth, mock_usdfc, global_faucet) addresses
pub fn check_required_addresses(
    context: &crate::commands::start::step::SetupContext,
) -> Result<(String, String, String, String), Box<dyn Error>> {
    let foc_deployer = context.get("deployer_foc_address").ok_or(
        "DEPLOYER_FOC address not found in context. Ensure ETHAccFunding step has been completed.",
    )?;

    let foc_deployer_eth = context
        .get("deployer_foc_eth_address")
        .ok_or("DEPLOYER_FOC Ethereum address not found in context. Ensure ETHAccFunding step has been completed.")?;

    let mock_usdfc = context.get("mockusdfc_contract_address").ok_or(
        "MockUSDFC address not found in context. Ensure USDFCDeploy step has been completed.",
    )?;

    let global_faucet = context
        .get("global_faucet_address")
        .ok_or("GLOBAL_FIL_FAUCET address not found in context. Ensure ETHAccFunding step has been completed.")?;

    Ok((
        foc_deployer.clone(),
        foc_deployer_eth.clone(),
        mock_usdfc.clone(),
        global_faucet.clone(),
    ))
}
