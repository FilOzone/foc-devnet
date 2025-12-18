//! Multicall3 Contract Deployment step implementation.
//!
//! This module contains the MultiCall3DeployStep struct and its implementation
//! of the Step trait for deploying the Multicall3 contract.

use super::super::step::{SetupContext, Step};
use super::deployment::perform_deployment;
use super::prerequisites::{
    check_existing_deployment, check_lotus_running, check_required_addresses,
};
use std::error::Error;
use std::path::PathBuf;
use tracing::info;

/// Step for deploying Multicall3 contract
pub struct MultiCall3DeployStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    run_dir: PathBuf,
}

impl MultiCall3DeployStep {
    /// Create a new MultiCall3DeployStep
    pub fn new(volumes_dir: PathBuf, run_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            run_dir,
        }
    }
}

impl Step for MultiCall3DeployStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Deploy Multicall3 Contract"
    }

    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        check_lotus_running(context)?;
        info!("    ✓ Lotus is running");

        // Check if required addresses are available
        let (multicall3_deployer, multicall3_deployer_eth) = check_required_addresses(context)?;
        info!("    ✓ DEPLOYER_MULTICALL3 address: {}", multicall3_deployer);
        info!(
            "    ✓ DEPLOYER_MULTICALL3 Ethereum address: {}",
            multicall3_deployer_eth
        );

        Ok(())
    }

    /// Execute the contract deployment process
    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        if check_existing_deployment(context) {
            info!("    ✓ Multicall3 contract already deployed, skipping...");
            return Ok(());
        }

        perform_deployment(&self.volumes_dir, context)?;
        Ok(())
    }

    /// Perform post-execution verification for contract deployment
    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        info!("    Verifying Multicall3 deployment...");

        // Check if contract address is in context
        if let Some(contract_address) = context.get("multicall3_address") {
            info!("      ✓ Multicall3 address: {}", contract_address);
        } else {
            return Err("Multicall3 deployment failed - no address in context".into());
        }

        info!("    ✓ Multicall3 deployment step completed!");
        info!("      Contract is ready for use.");

        Ok(())
    }
}
