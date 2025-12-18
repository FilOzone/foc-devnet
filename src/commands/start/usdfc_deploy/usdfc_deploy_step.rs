//! MockUSDFC Token Deployment step implementation.
//!
//! This module contains the USDFCDeployStep struct and its implementation
//! of the Step trait for deploying the MockUSDFC ERC-20 token.

use super::super::step::{SetupContext, Step};
use super::deployment::perform_token_deployment;
use super::prerequisites::{
    check_existing_deployment, check_lotus_running, check_required_addresses,
};
use std::error::Error;
use std::path::PathBuf;
use tracing::info;

// Token configuration
pub const MOCK_USDFC_INITIAL_SUPPLY: &str = "1000000000000000000000000"; // 1 million tokens (18 decimals)

/// Step for deploying MockUSDFC token
pub struct USDFCDeployStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    run_dir: PathBuf,
}

impl USDFCDeployStep {
    /// Create a new USDFCDeployStep
    pub fn new(volumes_dir: PathBuf, run_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            run_dir,
        }
    }
}

impl Step for USDFCDeployStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Deploy MockUSDFC Token"
    }

    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        check_lotus_running(context)?;
        info!("    ✓ Lotus is running");

        // Check if required addresses are available
        let (mockusdfc_deployer, mockusdfc_deployer_eth) = check_required_addresses(context)?;
        info!("    ✓ DEPLOYER_MOCKUSDFC address: {}", mockusdfc_deployer);
        info!(
            "    ✓ DEPLOYER_MOCKUSDFC Ethereum address: {}",
            mockusdfc_deployer_eth
        );

        Ok(())
    }

    /// Execute the token deployment process
    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        if check_existing_deployment(context) {
            info!("    ✓ MockUSDFC token already deployed, skipping...");
            return Ok(());
        }

        perform_token_deployment(&self.volumes_dir, context)?;
        Ok(())
    }

    /// Perform post-execution verification for token deployment
    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        info!("    Verifying MockUSDFC deployment...");

        // Check if token address is in context
        if let Some(token_address) = context.get("mockusdfc_contract_address") {
            info!("      ✓ MockUSDFC address: {}", token_address.as_str());
        } else {
            info!("      ✗ MockUSDFC address not found in context");
            return Err("MockUSDFC deployment failed - no address in context".into());
        }

        info!("    ✓ MockUSDFC deployment step completed!");
        info!("      Token is ready for FOC contract deployment.");

        Ok(())
    }
}
