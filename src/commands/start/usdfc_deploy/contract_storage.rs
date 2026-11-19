//! Contract address storage for MockUSDFC deployment.
//!
//! This module handles saving deployed contract addresses to persistent storage.

use crate::commands::start::foc_deploy::contract_addresses::ContractAddresses;
use std::error::Error;
use tracing::info;

/// Save contract address to the contract addresses file.
///
/// Delegates to [`ContractAddresses::update`] so parallel deployment steps
/// merge their writes instead of clobbering each other.
pub fn save_contract_address(
    run_id: &str,
    name: &str,
    address: &str,
) -> Result<(), Box<dyn Error>> {
    ContractAddresses::update(run_id, |addresses| {
        addresses
            .contracts
            .insert(name.to_string(), address.to_string());
    })?;

    info!(
        "Contract address '{}' saved to {}",
        name,
        crate::paths::contract_addresses_file(run_id).display()
    );

    Ok(())
}
