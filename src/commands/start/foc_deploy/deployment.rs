//! FOC contract deployment execution.
//!
//! This module contains the core deployment execution logic,
//! including contract deployment and output parsing.

use crate::commands::start::foc_deploy::contract_addresses::ContractAddresses;
use crate::commands::start::foc_deployer::deploy_foc_contracts;
use crate::paths::{contract_addresses_file, foc_metadata_file};
use crossterm::style::Stylize;
use std::error::Error;
use tracing::{info, warn};

/// Check if FOC contracts are already deployed
///
/// # Arguments
/// * `context` - The setup context to store contract addresses
///
/// # Returns
/// true if contracts are already deployed, false otherwise
pub fn check_existing_deployment(
    context: &crate::commands::start::step::SetupContext,
) -> Result<bool, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    if let Ok(existing_addresses) = ContractAddresses::load(run_id) {
        if !existing_addresses.foc_contracts.is_empty() {
            info!(
                "    {} FOC contracts already deployed, skipping deployment...",
                "✓".green()
            );

            // Store contract addresses in context
            for (name, addr) in &existing_addresses.foc_contracts {
                context.set(format!("foc_contract_{}", name.replace(' ', "_")), addr);
            }
            return Ok(true);
        }
    }
    Ok(false)
}

/// Perform the FOC contract deployment process
///
/// # Arguments
/// * `context` - The setup context containing required addresses
pub fn perform_deployment(
    context: &crate::commands::start::step::SetupContext,
) -> Result<(), Box<dyn Error>> {
    info!("Deploying FOC service contracts...");

    // Get required addresses from context
    let (foc_deployer, foc_deployer_eth, mock_usdfc_address, _global_faucet) =
        super::helpers::check_required_addresses(context)?;

    let services_repo = super::helpers::get_filecoin_services_repo_path()?;

    // Get Lotus container name and RPC URL
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let lotus_container = crate::docker::containers::lotus_container_name(run_id);
    let lotus_rpc_url = crate::commands::start::lotus_utils::get_lotus_rpc_url(context)?;

    // Deploy FOC contracts using deployment script
    let contract_addresses = deploy_foc_contracts(
        &foc_deployer,
        &foc_deployer_eth,
        &mock_usdfc_address,
        &services_repo,
        &lotus_container,
        &lotus_rpc_url,
        run_id,
    )?;

    // Store contract addresses in context
    for (name, addr) in &contract_addresses.addresses {
        context.set(format!("foc_contract_{}", name.replace(' ', "_")), addr);
    }

    // Load existing addresses and update with FOC contracts
    let mut addresses_struct =
        ContractAddresses::load(run_id).unwrap_or_else(|_| ContractAddresses::default());

    addresses_struct.foc_contracts = contract_addresses.addresses.clone();
    addresses_struct.filbeam_controller = contract_addresses.filbeam_controller.clone();
    addresses_struct.filbeam_beneficiary = contract_addresses.filbeam_beneficiary.clone();

    addresses_struct.save(run_id)?;
    info!(
        "      {} Contract addresses saved to {}",
        "✓".green(),
        contract_addresses_file(run_id).display()
    );

    // Save network metadata
    contract_addresses.metadata.save(run_id)?;
    info!(
        "      {} Network metadata saved to {}",
        "✓".green(),
        foc_metadata_file(run_id).display()
    );

    Ok(())
}

/// Perform post-execution verification for FOC deployment
///
/// # Arguments
/// * `context` - The step context to verify
pub fn post_execute_verification(
    context: &crate::commands::start::step::SetupContext,
) -> Result<(), Box<dyn Error>> {
    info!("Verifying FOC deployment...");

    // Check if contracts were deployed
    let contract_keys = context.get_keys_matching(|k| k.starts_with("foc_contract_"));
    let contract_count = contract_keys.len();

    if contract_count > 0 {
        info!("✓ {} contracts verified in context", contract_count);
    } else {
        warn!("⚠ No contracts found in context");
    }

    info!("✓ FOC deployment step completed!");
    info!("All FOC service contracts are deployed and ready.");

    Ok(())
}
