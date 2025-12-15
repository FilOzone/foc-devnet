//! Prerequisites checking for Multicall3 deployment.
//!
//! This module contains functions that verify all prerequisites are met
//! before deploying the Multicall3 contract.

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
            "Lotus container '{}' is not running. Multicall3 deployment requires Lotus to be running with FEVM enabled.",
            lotus_name
        )
        .into());
    }

    Ok(())
}

/// Check if required addresses are available in context
pub fn check_required_addresses(context: &StepContext) -> Result<(String, String), Box<dyn Error>> {
    let multicall3_deployer = context.get("deployer_multicall3_address").ok_or(
        "DEPLOYER_MULTICALL3 address not found in context. Ensure ETHAccFunding step has been completed.",
    )?;

    let multicall3_deployer_eth = context
        .get("deployer_multicall3_eth_address")
        .ok_or("DEPLOYER_MULTICALL3 Ethereum address not found in context. Ensure ETHAccFunding step has been completed.")?;

    Ok((multicall3_deployer.clone(), multicall3_deployer_eth.clone()))
}

/// Check if Multicall3 has already been deployed
pub fn check_existing_deployment(context: &StepContext) -> bool {
    context.get("multicall3_address").is_some()
}
