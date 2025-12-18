//! MockUSDFC Token Deployment step implementation.
//!
//! This module contains the USDFCDeployStep struct and its implementation
//! of the Step trait for deploying the MockUSDFC ERC-20 token.

use super::super::step::{Step, StepContext};
use super::deployment::perform_token_deployment;
use super::prerequisites::{
    check_existing_deployment, check_lotus_running, check_required_addresses,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;

// Token configuration
pub const MOCK_USDFC_INITIAL_SUPPLY: &str = "1000000000000000000000000"; // 1 million tokens (18 decimals)

/// Step for deploying MockUSDFC token
pub struct USDFCDeployStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl USDFCDeployStep {
    /// Create a new USDFCDeployStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }
}

impl Step for USDFCDeployStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Deploy MockUSDFC Token"
    }

    fn pre_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        check_lotus_running(context)?;
        println!("    {} Lotus is running", "✓".green());

        // Check if required addresses are available
        let (mockusdfc_deployer, mockusdfc_deployer_eth) = check_required_addresses(context)?;
        println!(
            "    {} DEPLOYER_MOCKUSDFC address: {}",
            "✓".green(),
            mockusdfc_deployer.cyan()
        );
        println!(
            "    {} DEPLOYER_MOCKUSDFC Ethereum address: {}",
            "✓".green(),
            mockusdfc_deployer_eth.cyan()
        );

        Ok(())
    }

    /// Execute the token deployment process
    fn execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        if check_existing_deployment(context) {
            println!(
                "    {} MockUSDFC token already deployed, skipping...",
                "✓".green()
            );
            return Ok(());
        }

        perform_token_deployment(&self.volumes_dir, context)?;
        Ok(())
    }

    /// Perform post-execution verification for token deployment
    fn post_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying MockUSDFC deployment...");

        // Check if token address is in context
        if let Some(token_address) = context.get("mock_usdfc_address") {
            println!(
                "      {} MockUSDFC address: {}",
                "✓".green(),
                token_address.as_str().cyan().bold()
            );
        } else {
            println!("      {} MockUSDFC address not found in context", "✗".red());
            return Err("MockUSDFC deployment failed - no address in context".into());
        }

        println!(
            "\n    {} MockUSDFC deployment step completed!",
            "✓".green().bold()
        );
        println!("      Token is ready for FOC contract deployment.");

        Ok(())
    }
}
