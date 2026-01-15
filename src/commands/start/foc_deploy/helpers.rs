//! FOC deployment helper functions.
//!
//! This module contains helper functions for FOC contract deployment,
//! including repository path resolution and deployment checks.

use crate::config::{Config, Location};
use crate::constants::LOCAL_NETWORK_CHAIN_ID;
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use crate::paths::{foc_devnet_config, foc_devnet_filecoin_services_repo};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tracing::info;

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

/// Clear cached devnet deployment addresses from deployments.json
///
/// The filecoin-services deployment script reads deployments.json and skips
/// deployment if addresses already exist for the chain ID. This causes issues
/// when starting a fresh devnet because the cached addresses point to contracts
/// that don't exist on the new chain. This function removes the devnet entry
/// so that contracts are actually deployed.
pub fn clear_cached_devnet_deployments() -> Result<(), Box<dyn Error>> {
    let services_repo = get_filecoin_services_repo_path()?;
    let deployments_file = services_repo.join("service_contracts/deployments.json");

    if !deployments_file.exists() {
        info!("No deployments.json found, skipping cache clear");
        return Ok(());
    }

    let content = fs::read_to_string(&deployments_file)?;
    let mut deployments: serde_json::Value = serde_json::from_str(&content)?;

    let chain_id_str = LOCAL_NETWORK_CHAIN_ID.to_string();
    if let Some(obj) = deployments.as_object_mut() {
        if obj.remove(&chain_id_str).is_some() {
            info!(
                "Cleared cached devnet (chain {}) addresses from deployments.json",
                chain_id_str
            );
            let updated = serde_json::to_string_pretty(&deployments)?;
            fs::write(&deployments_file, updated)?;
        } else {
            info!("No cached devnet addresses found in deployments.json");
        }
    }

    Ok(())
}
