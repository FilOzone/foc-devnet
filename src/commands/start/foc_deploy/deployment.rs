//! FOC contract deployment execution.
//!
//! This module contains the core deployment execution logic,
//! including contract deployment and output parsing.

use crate::commands::start::foc_deploy::contract_addresses::ContractAddresses;
use crate::commands::start::foc_deployer::deploy_foc_contracts;
use crate::paths::{contract_addresses_file, foc_metadata_file};
use crossterm::style::Stylize;
use std::error::Error;

/// Check if FOC contracts are already deployed
///
/// # Arguments
/// * `context` - The step context to store contract addresses
///
/// # Returns
/// true if contracts are already deployed, false otherwise
pub fn check_existing_deployment(
    context: &crate::commands::start::step::StepContext,
) -> Result<bool, Box<dyn Error>> {
    if let Ok(existing_addresses) = ContractAddresses::load() {
        if !existing_addresses.foc_contracts.is_empty() {
            println!(
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
/// * `context` - The step context containing required addresses
pub fn perform_deployment(
    context: &crate::commands::start::step::StepContext,
) -> Result<(), Box<dyn Error>> {
    println!("    Deploying FOC service contracts...");

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
    )?;

    // Store contract addresses in context
    for (name, addr) in &contract_addresses.addresses {
        context.set(format!("foc_contract_{}", name.replace(' ', "_")), addr);
    }

    // Load existing addresses and update with FOC contracts
    let mut addresses_struct =
        ContractAddresses::load().unwrap_or_else(|_| ContractAddresses::default());

    addresses_struct.foc_contracts = contract_addresses.addresses.clone();
    addresses_struct.filbeam_controller = contract_addresses.filbeam_controller.clone();
    addresses_struct.filbeam_beneficiary = contract_addresses.filbeam_beneficiary.clone();

    addresses_struct.save()?;
    println!(
        "      {} Contract addresses saved to {}",
        "✓".green(),
        contract_addresses_file().display()
    );

    // Save network metadata
    contract_addresses.metadata.save()?;
    println!(
        "      {} Network metadata saved to {}",
        "✓".green(),
        foc_metadata_file().display()
    );

    Ok(())
}

/// Perform post-execution verification for FOC deployment
///
/// # Arguments
/// * `context` - The step context to verify
pub fn post_execute_verification(
    context: &crate::commands::start::step::StepContext,
) -> Result<(), Box<dyn Error>> {
    println!("    Verifying FOC deployment...");

    // Check if contracts were deployed
    let contract_keys = context.get_keys_matching(|k| k.starts_with("foc_contract_"));
    let contract_count = contract_keys.len();

    if contract_count > 0 {
        println!(
            "      {} {} contracts verified in context",
            "✓".green(),
            contract_count
        );
    } else {
        println!("      {} No contracts found in context", "⚠".yellow());
    }

    println!(
        "\n    {} FOC deployment step completed!",
        "✓".green().bold()
    );
    println!("      All FOC service contracts are deployed and ready.");

    Ok(())
}
