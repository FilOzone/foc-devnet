//! Ethereum Account Funding step implementation.
//!
//! This module contains the main Step implementation for funding Ethereum accounts.

use super::constants::{FEVM_FAUCET_AMOUNT, FOC_DEPLOYER_AMOUNT, GLOBAL_FIL_FAUCET_KEY};
use super::funding_operations::transfer_fil;
use super::key_operations::{
    create_fevm_address, export_private_key, get_eth_address, import_faucet_key,
};
use super::lotus_checks::{check_lotus_running, get_global_faucet_address};
use crate::commands::start::step::{Step, StepContext};
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Step for funding Ethereum accounts required for FOC deployment
pub struct ETHAccFundingStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl ETHAccFundingStep {
    /// Create a new ETHAccFundingStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }

    /// Check if account funding has already been completed
    fn check_existing_funding(&self, context: &mut StepContext) -> Result<bool, Box<dyn Error>> {
        // Check if we have the required addresses in context
        let has_global_faucet = context.get("global_faucet_address").is_some();
        let has_fevm_faucet = context.get("fevm_faucet_address").is_some();
        let has_foc_deployer = context.get("foc_deployer_address").is_some();
        let has_eth_address = context.get("foc_deployer_eth_address").is_some();

        if has_global_faucet && has_fevm_faucet && has_foc_deployer && has_eth_address {
            println!(
                "    {} Account funding already completed, skipping...",
                "✓".green()
            );
            return Ok(true);
        }

        Ok(false)
    }

    /// Perform the account funding process
    fn perform_account_funding(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Setting up Ethereum accounts for FOC deployment...");

        // Step 1: Import GLOBAL_FIL_FAUCET key
        let keys_dir = crate::paths::foc_localnet_lotus_keys();
        let faucet_key_dir = keys_dir.join(GLOBAL_FIL_FAUCET_KEY);
        let keyinfo_files: Vec<_> = fs::read_dir(&faucet_key_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.starts_with("bls-") && s.ends_with(".keyinfo"))
                    .unwrap_or(false)
            })
            .collect();

        if keyinfo_files.is_empty() {
            return Err("No keyinfo file found for GLOBAL_FIL_FAUCET".into());
        }

        let keyinfo_path = keyinfo_files[0].path();
        let global_faucet = import_faucet_key(&keyinfo_path)?;
        context.set("global_faucet_address", &global_faucet);

        // Step 2: Create FEVM_FAUCET address
        let fevm_faucet = create_fevm_address("FEVM_FAUCET")?;
        context.set("fevm_faucet_address", &fevm_faucet);

        // Step 3: Transfer FIL from GLOBAL_FIL_FAUCET to FEVM_FAUCET
        transfer_fil(
            &global_faucet,
            &fevm_faucet,
            FEVM_FAUCET_AMOUNT,
            "GLOBAL_FIL_FAUCET → FEVM_FAUCET",
        )?;

        // Step 4: Create FOC_DEPLOYER address
        let foc_deployer = create_fevm_address("FOC_DEPLOYER")?;
        context.set("foc_deployer_address", &foc_deployer);

        // Step 5: Transfer FIL from FEVM_FAUCET to FOC_DEPLOYER
        transfer_fil(
            &fevm_faucet,
            &foc_deployer,
            FOC_DEPLOYER_AMOUNT,
            "FEVM_FAUCET → FOC_DEPLOYER",
        )?;

        // Step 6: Get Ethereum address for FOC_DEPLOYER
        let deployer_eth_addr = get_eth_address(&foc_deployer)?;
        println!(
            "      {} FOC_DEPLOYER Ethereum address: {}",
            "✓".green(),
            deployer_eth_addr
        );
        context.set("foc_deployer_eth_address", &deployer_eth_addr);

        // Step 7: Export private key for FOC_DEPLOYER
        let deployer_key_file = self.volumes_dir.join("foc-deployer.key");
        export_private_key(&foc_deployer, &deployer_key_file)?;

        println!(
            "\n    {} Ethereum accounts funded successfully!",
            "✓".green().bold()
        );
        println!("      GLOBAL_FIL_FAUCET: {}", global_faucet);
        println!("      FEVM_FAUCET: {}", fevm_faucet);
        println!("      FOC_DEPLOYER: {}", foc_deployer);
        println!("      FOC_DEPLOYER (ETH): {}", deployer_eth_addr);

        Ok(())
    }
}

impl Step for ETHAccFundingStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Fund Ethereum Accounts"
    }

    fn pre_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        check_lotus_running()?;
        println!("    {} Lotus is running", "✓".green());

        // Check if GLOBAL_FIL_FAUCET key exists
        let faucet_addr = get_global_faucet_address()?;
        println!(
            "    {} GLOBAL_FIL_FAUCET address: {}",
            "✓".green(),
            faucet_addr
        );

        Ok(())
    }

    /// Execute the account funding process
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        if self.check_existing_funding(context)? {
            return Ok(());
        }

        self.perform_account_funding(context)?;
        Ok(())
    }

    /// Perform post-execution verification for account funding
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying account funding...");

        // Check if all required addresses are in context
        let required_keys = vec![
            "global_faucet_address",
            "fevm_faucet_address",
            "foc_deployer_address",
            "foc_deployer_eth_address",
        ];

        for key in required_keys {
            if let Some(value) = context.get(key) {
                println!("      {} {}: {}", "✓".green(), key, value);
            } else {
                println!("      {} {} not found in context", "✗".red(), key);
                return Err(format!("Missing required address: {}", key).into());
            }
        }

        println!(
            "\n    {} Account funding step completed!",
            "✓".green().bold()
        );
        println!("      All Ethereum accounts are funded and ready for contract deployment.");

        Ok(())
    }
}
