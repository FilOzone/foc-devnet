//! Ethereum Account Funding step implementation.
//!
//! This module contains the main Step implementation for funding Ethereum accounts.

use super::constants::GLOBAL_FIL_FAUCET_KEY;
use super::funding_operations::transfer_fil;
use super::key_operations::import_faucet_key;
use super::lotus_checks::{check_lotus_running, get_global_faucet_address};
use crate::commands::init::keys::load_keys;
use crate::commands::start::eth_acc_funding::constants::FEVM_ACCOUNTS_PREFUNDED;
use crate::commands::start::step::{Step, StepContext};
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Step for funding Ethereum accounts required for FOC deployment
pub struct ETHAccFundingStep {
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl ETHAccFundingStep {
    /// Create a new ETHAccFundingStep
    pub fn new(logs_dir: PathBuf) -> Self {
        Self { logs_dir }
    }

    /// Check if account funding has already been completed
    fn check_existing_funding(&self, context: &mut StepContext) -> Result<bool, Box<dyn Error>> {
        // Check if we have the required addresses in context
        let has_global_faucet = context.get("global_faucet_address").is_some();
        let has_all_prefunded_accounts = FEVM_ACCOUNTS_PREFUNDED.iter().all(|(name, _)| {
            context
                .get(&format!("{}_address", name.to_lowercase()))
                .is_some()
        });

        if has_global_faucet && has_all_prefunded_accounts {
            println!(
                "    {} Account funding already completed, skipping...",
                "✓".green()
            );
            return Ok(true);
        }

        Ok(false)
    }

    fn import_global_faucet_key(context: &StepContext) -> Result<String, Box<dyn Error + 'static>> {
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
        let global_faucet = import_faucet_key(&keyinfo_path, context)?;
        Ok(global_faucet)
    }

    /// Export key for use with Foundry (without importing to Lotus)
    fn export_key_for_foundry_direct(
        private_key: &str,
        volumes_dir: &PathBuf,
        key_file_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        use base64::{engine::general_purpose, Engine as _};

        // Remove 0x prefix if present
        let pk_hex = private_key.strip_prefix("0x").unwrap_or(private_key);
        let pk_bytes = hex::decode(pk_hex)?;

        // Create a keyinfo structure for secp256k1 (Ethereum-compatible) keys
        let keyinfo = serde_json::json!({
            "Type": "secp256k1",
            "PrivateKey": general_purpose::STANDARD.encode(&pk_bytes)
        });

        let keyinfo_json = serde_json::to_string(&keyinfo)?;
        let hex_encoded = hex::encode(keyinfo_json);

        let key_file = volumes_dir.join(format!("{}.key", key_file_name));
        fs::write(&key_file, hex_encoded)?;
        Ok(())
    }

    /// Perform the account funding process using pre-generated keys from keys.rs
    ///
    /// Account funding goes as follows:
    /// GLOBAL_FIL_FAUCET (has FIL from genesis, imported from BLS key)
    ///  → All pre-funded FEVM accounts (using addresses from keys.rs, NOT imported to Lotus)
    fn perform_account_funding(
        &self,
        context: &mut StepContext,
        volumes_dir: &PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        println!("    Setting up Ethereum accounts for FOC deployment...");

        // Load pre-generated keys from keys.rs
        let keys = load_keys()?;

        // Import GLOBAL_FIL_FAUCET key (BLS key from genesis) - this is the ONLY key imported to Lotus
        let global_faucet = Self::import_global_faucet_key(context)?;
        context.set("global_faucet_address", &global_faucet);

        // Fund all FEVM accounts from keys.rs (using pre-calculated addresses, NOT importing to Lotus)
        for (account_name, amount) in FEVM_ACCOUNTS_PREFUNDED.iter() {
            // Find the key info for this account
            let key_info = keys
                .iter()
                .find(|k| k.name == *account_name)
                .ok_or(format!("Key not found for account: {}", account_name))?;

            // Get addresses directly from keys.rs (no Lotus wallet import needed!)
            let f4_address = key_info
                .filecoin_address
                .as_ref()
                .ok_or(format!("No Filecoin address for {}", account_name))?;
            let eth_address = key_info
                .eth_address
                .as_ref()
                .ok_or(format!("No Ethereum address for {}", account_name))?;

            println!(
                "      {} {}: {} (ETH: {})",
                "✓".green(),
                account_name,
                f4_address,
                eth_address
            );

            // Store in context using snake_case
            let address_key = format!("{}_address", account_name.to_lowercase());
            let eth_key = format!("{}_eth_address", account_name.to_lowercase());
            context.set(&address_key, f4_address);
            context.set(&eth_key, eth_address);

            // Export key for Foundry (for deployers only) - no Lotus needed!
            if account_name.ends_with("_DEPLOYER") {
                let key_file_name = account_name.to_lowercase().replace('_', "-");
                Self::export_key_for_foundry_direct(
                    &key_info.private_key,
                    volumes_dir,
                    &key_file_name,
                )?;
            }

            // Transfer FIL from GLOBAL_FIL_FAUCET to this account
            transfer_fil(
                &global_faucet,
                f4_address,
                *amount,
                &format!("GLOBAL_FIL_FAUCET → {}", account_name),
                context,
            )?;
        }

        Ok(())
    }
}

impl Step for ETHAccFundingStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Fund Ethereum Accounts"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        check_lotus_running(context)?;
        println!("    {} Lotus is running", "✓".green());

        // Check if GLOBAL_FIL_FAUCET key exists
        let faucet_addr = get_global_faucet_address()?;
        println!(
            "    {} GLOBAL_FIL_FAUCET address: {}",
            "✓".green(),
            faucet_addr
        );

        // Check if keys.rs keys are available
        let keys = load_keys()?;
        println!(
            "    {} Loaded {} pre-generated keys",
            "✓".green(),
            keys.len()
        );

        Ok(())
    }

    /// Execute the account funding process
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        if self.check_existing_funding(context)? {
            return Ok(());
        }

        let volumes_dir = crate::paths::foc_localnet_docker_volumes();
        self.perform_account_funding(context, &volumes_dir)?;
        Ok(())
    }

    /// Perform post-execution verification for account funding
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying account funding...");

        // Check if all FEVM accounts are in context
        for (account_name, _) in FEVM_ACCOUNTS_PREFUNDED.iter() {
            let address_key = format!("{}_address", account_name.to_lowercase());
            let eth_key = format!("{}_eth_address", account_name.to_lowercase());

            if let Some(addr) = context.get(&address_key) {
                println!("      {} {}: {}", "✓".green(), account_name, addr);
            } else {
                return Err(format!("Missing address for: {}", account_name).into());
            }

            if let Some(eth_addr) = context.get(&eth_key) {
                println!("      {} {} (ETH): {}", "✓".green(), account_name, eth_addr);
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
