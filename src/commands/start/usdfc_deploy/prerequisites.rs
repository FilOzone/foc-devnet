//! Prerequisites checking for MockUSDFC deployment.
//!
//! This module contains functions that verify all prerequisites are met
//! before deploying the MockUSDFC token.

use super::super::step::StepContext;
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use std::error::Error;

/// Check if Lotus is running and accessible
pub fn check_lotus_running(context: &StepContext) -> Result<(), Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
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
pub fn check_required_addresses(context: &StepContext) -> Result<(String, String), Box<dyn Error>> {
    let mockusdfc_deployer = context.get("deployer_mockusdfc_address").ok_or(
        "DEPLOYER_MOCKUSDFC address not found in context. Ensure ETHAccFunding step has been completed.",
    )?;

    let mockusdfc_deployer_eth = context
        .get("deployer_mockusdfc_eth_address")
        .ok_or("DEPLOYER_MOCKUSDFC Ethereum address not found in context. Ensure ETHAccFunding step has been completed.")?;

    Ok((mockusdfc_deployer.clone(), mockusdfc_deployer_eth.clone()))
}

/// Check if MockUSDFC has already been deployed
pub fn check_existing_deployment(context: &StepContext) -> bool {
    context.get("mock_usdfc_address").is_some()
}
