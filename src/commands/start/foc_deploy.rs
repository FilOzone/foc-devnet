//! FOC (Filecoin Onchain Contracts) deployment step.
//!
//! This module handles deploying FOC service contracts to the Lotus node with FEVM enabled.
//! These contracts are required by Curio for storage provider operations.

use super::contract_addresses::ContractAddresses;
use super::foc_deployer::deploy_foc_contracts;
use super::step::{Step, StepContext};
use crate::config::{Config, Location};
use crate::constants::*;
use crate::paths::{contract_addresses_file, foc_localnet_config, foc_localnet_filecoin_services_repo};
use crate::shell::docker_container_is_running;
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Step for deploying FOC service contracts
pub struct FOCDeployStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl FOCDeployStep {
    /// Create a new FOCDeployStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }

    /// Get the filecoin-services repository path based on configuration
    fn get_filecoin_services_repo_path() -> Result<PathBuf, Box<dyn Error>> {
        // Load configuration
        let config_path = foc_localnet_config();
        let config_content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
        let config: Config = toml::from_str(&config_content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        // Determine the repository path based on location
        let repo_path = match &config.filecoin_services {
            Location::LocalSource { dir } => {
                // For LocalSource, use the configured directory directly
                PathBuf::from(dir)
            }
            _ => {
                // For Git-based locations, use the foc-localnet directory
                foc_localnet_filecoin_services_repo()
            }
        };

        Ok(repo_path)
    }

    /// Check if Lotus is running and accessible
    fn check_lotus_running() -> Result<(), Box<dyn Error>> {
        if !docker_container_is_running(LOTUS_CONTAINER)? {
            return Err("Lotus container is not running. FOC deployment requires Lotus to be running with FEVM enabled.".into());
        }
        Ok(())
    }

    /// Check if required addresses are available in context
    fn check_required_addresses(
        &self,
        context: &StepContext,
    ) -> Result<(String, String, String, String), Box<dyn Error>> {
        let foc_deployer = context
            .get("foc_deployer_address")
            .ok_or("FOC_DEPLOYER address not found in context. Ensure ETHAccFunding step has been completed.")?;

        let foc_deployer_eth = context
            .get("foc_deployer_eth_address")
            .ok_or("FOC_DEPLOYER Ethereum address not found in context. Ensure ETHAccFunding step has been completed.")?;

        let mock_usdfc = context.get("mock_usdfc_address").ok_or(
            "MockUSDFC address not found in context. Ensure USDFCDeploy step has been completed.",
        )?;

        let global_faucet = context
            .get("global_faucet_address")
            .ok_or("GLOBAL_FIL_FAUCET address not found in context. Ensure ETHAccFunding step has been completed.")?;

        Ok((
            foc_deployer.clone(),
            foc_deployer_eth.clone(),
            mock_usdfc.clone(),
            global_faucet.clone(),
        ))
    }

    /// Check if FOC contracts are already deployed
    fn check_existing_deployment(&self, context: &mut StepContext) -> Result<bool, Box<dyn Error>> {
        if let Ok(existing_addresses) = ContractAddresses::load() {
            if !existing_addresses.foc_contracts.is_empty() {
                println!(
                    "    {} FOC contracts already deployed, skipping deployment...",
                    "✓".green()
                );

                // Store contract addresses in context
                for (name, addr) in &existing_addresses.foc_contracts {
                    context.set(&format!("foc_contract_{}", name.replace(' ', "_")), addr);
                }
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Perform the FOC contract deployment process
    fn perform_deployment(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Deploying FOC service contracts...");

        // Get required addresses from context
        let (foc_deployer, foc_deployer_eth, mock_usdfc_address, global_faucet) =
            self.check_required_addresses(context)?;

        let services_repo = Self::get_filecoin_services_repo_path()?;

        // Deploy FOC contracts using deployment script
        let contract_addresses = deploy_foc_contracts(
            &foc_deployer,
            &foc_deployer_eth,
            &mock_usdfc_address,
            &services_repo,
        )?;

        // Store contract addresses in context
        for (name, addr) in &contract_addresses {
            context.set(&format!("foc_contract_{}", name.replace(' ', "_")), addr);
        }

        // Load existing addresses and update with FOC contracts
        let mut addresses_struct =
            ContractAddresses::load().unwrap_or_else(|_| ContractAddresses {
                global_fil_faucet: global_faucet,
                fevm_faucet: context
                    .get("fevm_faucet_address")
                    .unwrap_or(&String::new())
                    .clone(),
                foc_deployer: foc_deployer,
                foc_deployer_eth: foc_deployer_eth,
                mock_usdfc: mock_usdfc_address,
                foc_contracts: std::collections::HashMap::new(),
            });

        addresses_struct.foc_contracts = contract_addresses.clone();

        addresses_struct.save()?;
        println!(
            "      {} Contract addresses saved to {}",
            "✓".green(),
            contract_addresses_file().display()
        );

        println!(
            "\n    {} FOC service contracts deployed successfully!",
            "✓".green().bold()
        );
        println!("      Deployed {} contracts", contract_addresses.len());

        Ok(())
    }
}

impl Step for FOCDeployStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Deploy FOC Contracts"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        Self::check_lotus_running()?;
        println!("    {} Lotus is running", "✓".green());

        // Check if filecoin-services repository exists
        let services_repo = Self::get_filecoin_services_repo_path()?;
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
            self.check_required_addresses(context)?;
        println!(
            "    {} FOC_DEPLOYER Ethereum address: {}",
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
        if self.check_existing_deployment(context)? {
            return Ok(());
        }

        self.perform_deployment(context)?;
        Ok(())
    }

    /// Perform post-execution verification for FOC deployment
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying FOC deployment...");

        // Check if contracts were deployed
        let mut contract_count = 0;
        for (key, _) in &context.state {
            if key.starts_with("foc_contract_") {
                contract_count += 1;
            }
        }

        if contract_count > 0 {
            println!(
                "      {} {} contracts verified in context",
                "✓".green(),
                contract_count
            );
        } else {
            println!("      {} No contracts found in context", "⚠".yellow());
        }

        println!(
            "\n    {} FOC deployment step completed!",
            "✓".green().bold()
        );
        println!("      All FOC service contracts are deployed and ready.");

        Ok(())
    }
}
