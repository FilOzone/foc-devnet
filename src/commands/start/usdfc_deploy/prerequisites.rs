//! Prerequisites checking for MockUSDFC deployment.
//!
//! This module contains functions that verify all prerequisites are met
//! before deploying the MockUSDFC token.

use super::super::step::StepContext;
use std::error::Error;
use std::process::Command;

/// Check if Lotus is running and accessible
pub fn check_lotus_running() -> Result<(), Box<dyn Error>> {
    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            "name=^foc-lotus$",
            "--format",
            "{{.Names}}",
        ])
        .output()?;

    if !String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains("foc-lotus")
    {
        return Err("Lotus container is not running. MockUSDFC deployment requires Lotus to be running with FEVM enabled.".into());
    }

    Ok(())
}

/// Check if required addresses are available in context
pub fn check_required_addresses(context: &StepContext) -> Result<(String, String), Box<dyn Error>> {
    let mockusdfc_deployer = context.get("mockusdfc_deployer_address").ok_or(
        "MOCKUSDFC_DEPLOYER address not found in context. Ensure ETHAccFunding step has been completed.",
    )?;

    let mockusdfc_deployer_eth = context
        .get("mockusdfc_deployer_eth_address")
        .ok_or("MOCKUSDFC_DEPLOYER Ethereum address not found in context. Ensure ETHAccFunding step has been completed.")?;

    Ok((mockusdfc_deployer.clone(), mockusdfc_deployer_eth.clone()))
}

/// Check if MockUSDFC has already been deployed
pub fn check_existing_deployment(context: &StepContext) -> bool {
    context.get("mock_usdfc_address").is_some()
}
