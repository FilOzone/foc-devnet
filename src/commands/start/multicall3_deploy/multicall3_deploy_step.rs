//! Multicall3 Contract Deployment step implementation.
//!
//! This module contains the MultiCall3DeployStep struct and its implementation
//! of the Step trait for deploying the Multicall3 contract.

use super::super::step::{Step, StepContext};
use super::deployment::perform_deployment;
use super::prerequisites::{
    check_existing_deployment, check_lotus_running, check_required_addresses,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;

/// Step for deploying Multicall3 contract
pub struct MultiCall3DeployStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl MultiCall3DeployStep {
    /// Create a new MultiCall3DeployStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }
}

impl Step for MultiCall3DeployStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Deploy Multicall3 Contract"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        check_lotus_running(context)?;
        println!("    {} Lotus is running", "✓".green());

        // Check if required addresses are available
        let (multicall3_deployer, multicall3_deployer_eth) = check_required_addresses(context)?;
        println!(
            "    {} DEPLOYER_MULTICALL3 address: {}",
            "✓".green(),
            multicall3_deployer.cyan()
        );
        println!(
            "    {} DEPLOYER_MULTICALL3 Ethereum address: {}",
            "✓".green(),
            multicall3_deployer_eth.cyan()
        );

        Ok(())
    }

    /// Execute the contract deployment process
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        if check_existing_deployment(context) {
            println!(
                "    {} Multicall3 contract already deployed, skipping...",
                "✓".green()
            );
            return Ok(());
        }

        perform_deployment(&self.volumes_dir, context)?;
        Ok(())
    }

    /// Perform post-execution verification for contract deployment
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying Multicall3 deployment...");

        // Check if contract address is in context
        if let Some(contract_address) = context.get("multicall3_address") {
            println!(
                "      {} Multicall3 address: {}",
                "✓".green(),
                contract_address.as_str().cyan().bold()
            );
        } else {
            println!(
                "      {} Multicall3 address not found in context",
                "✗".red()
            );
            return Err("Multicall3 deployment failed - no address in context".into());
        }

        println!(
            "\n    {} Multicall3 deployment step completed!",
            "✓".green().bold()
        );
        println!("      Contract is ready for use.");

        Ok(())
    }
}
