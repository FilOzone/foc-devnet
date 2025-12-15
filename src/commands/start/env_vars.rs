//! Environment variable configuration for containers.
//!
//! This module provides utilities for building environment variable
//! arguments for Docker containers in the FOC localnet.

use crate::commands::start::contract_addresses::ContractAddresses;
use crate::constants::*;
use std::error::Error;

/// Build network parameter environment variables.
///
/// These are required for all Lotus, Lotus-Miner, and Curio containers.
///
/// # Returns
/// Vector of `-e KEY=VALUE` pairs for Docker run command
pub fn build_network_env_vars() -> Vec<String> {
    // vec![
    //     "-e".to_string(),
    //     format!("{}={}", ENV_FOC_LOCALNET_CHAIN_ID, LOCAL_NETWORK_CHAIN_ID),
    //     "-e".to_string(),
    //     format!("{}={}", ENV_FOC_LOCALNET_BLOCK_DELAY, FOC_LOCALNET_BLOCK_DELAY),
    //     "-e".to_string(),
    //     format!("{}={}", ENV_FOC_LOCALNET_PROPAGATION_DELAY, FOC_LOCALNET_PROPAGATION_DELAY),
    //     "-e".to_string(),
    //     format!("{}={}", ENV_FOC_LOCALNET_EQUIVOCATION_DELAY, FOC_LOCALNET_EQUIVOCATION_DELAY),
    // ]
    vec![]
}

/// Build contract address environment variables for Curio.
///
/// These are only required for Curio containers and must be set after
/// contract deployment.
///
/// # Returns
/// Vector of `-e KEY=VALUE` pairs for Docker run command, or error if addresses not found
pub fn build_curio_contract_env_vars() -> Result<Vec<String>, Box<dyn Error>> {
    let addresses = ContractAddresses::load()?;

    let mut env_vars = Vec::new();

    // USDFC token address
    if let Some(addr) = addresses.contracts.get("usdfc") {
        env_vars.push("-e".to_string());
        env_vars.push(format!("{}={}", ENV_FOC_LOCALNET_CONTRACT_USDFC, addr));
    }

    // FilecoinWarmStorageService (FWSS) address
    if let Some(addr) = addresses
        .foc_contracts
        .get("filecoin_warm_storage_service_proxy")
    {
        env_vars.push("-e".to_string());
        env_vars.push(format!("{}={}", ENV_FOC_LOCALNET_CONTRACT_FWSS, addr));
    }

    // Multicall3 address
    if let Some(addr) = addresses.contracts.get("multicall") {
        env_vars.push("-e".to_string());
        env_vars.push(format!("{}={}", ENV_FOC_LOCALNET_CONTRACT_MULTICALL, addr));
    }

    // Simple service address (constant zero address)
    env_vars.push("-e".to_string());
    env_vars.push(format!(
        "{}={}",
        ENV_FOC_LOCALNET_CONTRACT_SIMPLE, FOC_LOCALNET_CONTRACT_SIMPLE
    ));

    // FilecoinPay address (if exists - map from ServiceProviderRegistry or similar)
    // Note: Based on existing code, we might need to adjust this mapping
    // For now, using ServiceProviderRegistry as the "pay" contract
    if let Some(addr) = addresses
        .foc_contracts
        .get("service_provider_registry_proxy")
    {
        env_vars.push("-e".to_string());
        env_vars.push(format!("{}={}", ENV_FOC_LOCALNET_CONTRACT_PAY, addr));
    }

    Ok(env_vars)
}
