//! FOC deployment step implementation.
//!
//! This module contains the FOCDeployStep struct and its implementation
//! of the Step trait for deploying FOC service contracts.

use super::deployment::{check_existing_deployment, perform_deployment, post_execute_verification};
use super::helpers::{
    check_lotus_running, check_required_addresses, get_filecoin_services_repo_path,
};
use crate::commands::start::step::{Step, StepContext};
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;

/// Step for deploying FOC service contracts
pub struct FOCDeployStep {
    #[allow(dead_code)]
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl FOCDeployStep {
    /// Create a new FOCDeployStep
    ///
    /// # Arguments
    /// * `volumes_dir` - Directory for Docker volumes
    /// * `logs_dir` - Directory for log files
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }
}

impl Step for FOCDeployStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Deploy FOC Contracts"
    }

    /// Perform pre-execution checks
    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        check_lotus_running(context)?;
        println!("    {} Lotus is running", "✓".green());

        let services_repo = get_filecoin_services_repo_path()?;
        if !services_repo.exists() {
            return Err(format!(
                "filecoin-services repository not found at {}. \
                 Please run 'foc-localnet init' to clone the repository.",
                services_repo.display()
            )
            .into());
        }
        println!("    {} filecoin-services repository found", "✓".green());

        // Check if deployment script exists
        let deploy_script = services_repo
            .join("service_contracts")
            .join("tools")
            .join("deploy-all-warm-storage.sh");

        if !deploy_script.exists() {
            return Err(
                format!("Deployment script not found at {}", deploy_script.display()).into(),
            );
        }
        println!("    {} Deployment script found", "✓".green());

        // Check if required addresses are available
        let (_foc_deployer, foc_deployer_eth, mock_usdfc, _global_faucet) =
            check_required_addresses(context)?;
        println!(
            "    {} DEPLOYER_FOC Ethereum address: {}",
            "✓".green(),
            foc_deployer_eth
        );
        println!(
            "    {} MockUSDFC token address: {}",
            "✓".green(),
            mock_usdfc
        );

        Ok(())
    }

    /// Execute the FOC deployment process
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        if check_existing_deployment(context)? {
            return Ok(());
        }

        perform_deployment(context)?;
        Ok(())
    }

    /// Perform post-execution verification for FOC deployment
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        post_execute_verification(context)
    }
}
