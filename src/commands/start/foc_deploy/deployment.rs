//! FOC contract deployment execution.
//!
//! This module contains the core deployment execution logic,
//! including contract deployment and output parsing.

use super::foc_deployer::{deploy_foc_contracts, parse_deployment_output};
use crate::constants::*;
use crate::paths::contract_addresses_file;
use crate::commands::start::contract_addresses::ContractAddresses;
use crossterm::style::Stylize;
use std::error::Error;

/// Check if FOC contracts are already deployed
///
/// # Arguments
/// * `context` - The step context to store contract addresses
///
/// # Returns
/// true if contracts are already deployed, false otherwise
pub fn check_existing_deployment(context: &mut super::step::StepContext) -> Result<bool, Box<dyn Error>> {
    if let Ok(existing_addresses) = ContractAddresses::load() {
        if !existing_addresses.foc_contracts.is_empty() {
            println!(
                "    {} FOC contracts already deployed, skipping deployment...",
                "✓".green()
            );

            // Store contract addresses in context
            for (name, addr) in &existing_addresses.foc_contracts {
                context.set(&format!("foc_contract_{}", name.replace(' ', "_")), addr);
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
pub fn perform_deployment(context: &mut super::step::StepContext) -> Result<(), Box<dyn Error>> {
    println!("    Deploying FOC service contracts...");

    // Get required addresses from context
    let (foc_deployer, foc_deployer_eth, mock_usdfc_address, global_faucet) =
        super::helpers::check_required_addresses(context)?;

    let services_repo = super::helpers::get_filecoin_services_repo_path()?;

    // Deploy FOC contracts using deployment script
    let contract_addresses = deploy_foc_contracts(
        &foc_deployer,
        &foc_deployer_eth,
        &mock_usdfc_address,
        &services_repo,
    )?;

    // Store contract addresses in context
    for (name, addr) in &contract_addresses {
        context.set(&format!("foc_contract_{}", name.replace(' ', "_")), addr);
    }

    // Load existing addresses and update with FOC contracts
    let mut addresses_struct =
        ContractAddresses::load().unwrap_or_else(|_| ContractAddresses {
            contracts: std::collections::HashMap::new(),
            foc_contracts: std::collections::HashMap::new(),
        });

    addresses_struct.foc_contracts = contract_addresses.clone();

    addresses_struct.save()?;
    println!(
        "      {} Contract addresses saved to {}",
        "✓".green(),
        contract_addresses_file().display()
    );

    println!(
        "\n    {} FOC service contracts deployed successfully!",
        "✓".green().bold()
    );
    println!("      Deployed {} contracts", contract_addresses.len());

    Ok(())
}

/// Perform post-execution verification for FOC deployment
///
/// # Arguments
/// * `context` - The step context to verify
pub fn post_execute_verification(context: &super::step::StepContext) -> Result<(), Box<dyn Error>> {
    println!("    Verifying FOC deployment...");

    // Check if contracts were deployed
    let mut contract_count = 0;
    for (key, _) in &context.state {
        if key.starts_with("foc_contract_") {
            contract_count += 1;
        }
    }

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