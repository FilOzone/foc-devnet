//! Prerequisites checking for MockUSDFC deployment.
//!
//! This module contains functions that verify all prerequisites are met
//! before deploying the MockUSDFC token.

use super::super::step::SetupContext;
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use std::error::Error;

/// Check if Lotus is running and accessible
pub fn check_lotus_running(context: &SetupContext) -> Result<(), Box<dyn Error>> {
    let run_id = context.run_id();
    let lotus_name = lotus_container_name(run_id);

    if !container_is_running(&lotus_name)? {
        return Err(format!(
            "Lotus container '{}' is not running. MockUSDFC deployment requires Lotus to be running with FEVM enabled.",
            lotus_name
        )
        .into());
    }

    Ok(())
}

/// Check if required addresses are available in context
pub fn check_required_addresses(
    context: &SetupContext,
) -> Result<(String, String), Box<dyn Error>> {
    let mockusdfc_deployer = context.get("deployer_mockusdfc_address").ok_or(
        "DEPLOYER_MOCKUSDFC address not found in context. Ensure ETHAccFunding step has been completed.",
    )?;

    let mockusdfc_deployer_eth = context
        .get("deployer_mockusdfc_eth_address")
        .ok_or("DEPLOYER_MOCKUSDFC Ethereum address not found in context. Ensure ETHAccFunding step has been completed.")?;

    Ok((mockusdfc_deployer.clone(), mockusdfc_deployer_eth.clone()))
}

/// Check if MockUSDFC has already been deployed
pub fn check_existing_deployment(context: &SetupContext) -> bool {
    context.get("mockusdfc_contract_address").is_some()
}
