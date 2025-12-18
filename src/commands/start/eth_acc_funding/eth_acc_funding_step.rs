//! Ethereum Account Funding step implementation.
//!
//! This module contains the main Step implementation for funding Ethereum accounts.

use super::constants::GLOBAL_FIL_FAUCET_KEY;
use super::key_operations::import_faucet_key;
use super::lotus_checks::{check_lotus_running, get_global_faucet_address};
use crate::commands::init::keys::load_keys;
use crate::commands::start::eth_acc_funding::constants::FEVM_ACCOUNTS_PREFUNDED;
use crate::commands::start::step::{Step, StepContext};
use crate::docker::containers::lotus_container_name;
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
    fn check_existing_funding(&self, context: &StepContext) -> Result<bool, Box<dyn Error>> {
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
        context: &StepContext,
        volumes_dir: &PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        println!("    Setting up Ethereum accounts for FOC deployment...");

        // Load pre-generated keys from keys.rs
        let keys = load_keys()?;

        // Import GLOBAL_FIL_FAUCET key (BLS key from genesis) - this is the ONLY key imported to Lotus
        let global_faucet = Self::import_global_faucet_key(context)?;
        context.set("global_faucet_address", &global_faucet);

        // Prepare all accounts first (setup phase)
        let mut account_transfers = Vec::new();
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
            if account_name.starts_with("DEPLOYER_") {
                let key_file_name = account_name.to_lowercase().replace('_', "-");
                Self::export_key_for_foundry_direct(
                    &key_info.private_key,
                    volumes_dir,
                    &key_file_name,
                )?;
            }

            // Collect transfer info for parallel execution
            account_transfers.push((account_name.to_string(), f4_address.to_string(), *amount));
        }

        // Execute all FIL transfers in parallel
        self.parallel_transfer_fil(&global_faucet, account_transfers, context)?;

        Ok(())
    }

    /// Execute multiple FIL transfers in parallel
    ///
    /// This function spawns a thread for each transfer to execute them concurrently.
    /// If any transfer fails, the entire operation fails after all threads complete.
    fn parallel_transfer_fil(
        &self,
        from: &str,
        transfers: Vec<(String, String, u64)>,
        context: &StepContext,
    ) -> Result<(), Box<dyn Error>> {
        use super::constants::TRANSACTION_CONFIRMATION_WAIT_SECS;

        let num_transfers = transfers.len();
        println!(
            "      Executing {} FIL transfers in parallel...",
            num_transfers
        );

        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        let container_name = lotus_container_name(run_id);
        let from_addr = from.to_string();

        // Shared error collection
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];

        for (account_name, to_addr, amount) in transfers {
            let container = container_name.clone();
            let from = from_addr.clone();
            let errors_clone = Arc::clone(&errors);

            let handle = thread::spawn(move || {
                let description = format!("GLOBAL_FIL_FAUCET → {}", account_name);
                println!("      Transferring {} FIL: {}...", amount, description);

                let output = Command::new("docker")
                    .args([
                        "exec",
                        &container,
                        "/usr/local/bin/lotus-bins/lotus",
                        "send",
                        "--from",
                        &from,
                        &to_addr,
                        &amount.to_string(),
                    ])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        println!(
                            "      ✓ Transferred {} FIL: {}",
                            amount,
                            description.dark_green().bold()
                        );
                    }
                    Ok(out) => {
                        let error_msg = format!(
                            "Failed to transfer {} FIL to {}: {}",
                            amount,
                            account_name,
                            String::from_utf8_lossy(&out.stderr)
                        );
                        eprintln!("      ✗ {}", error_msg.clone().red());
                        errors_clone.lock().unwrap().push(error_msg);
                    }
                    Err(e) => {
                        let error_msg =
                            format!("Failed to execute transfer to {}: {}", account_name, e);
                        eprintln!("      ✗ {}", error_msg.clone().red());
                        errors_clone.lock().unwrap().push(error_msg);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all transfers to complete
        for handle in handles {
            handle
                .join()
                .map_err(|_| "Thread panicked during transfer")?;
        }

        // Wait for transaction confirmation and address activation
        println!("      Waiting for transaction confirmations and address activations...");
        thread::sleep(Duration::from_secs(TRANSACTION_CONFIRMATION_WAIT_SECS * 2));

        // Check if any errors occurred
        let errors_vec = errors.lock().unwrap();
        if !errors_vec.is_empty() {
            let combined_error = errors_vec.join("\n");
            return Err(format!("One or more transfers failed:\n{}", combined_error).into());
        }

        println!(
            "      {} All {} transfers completed successfully!",
            "✓".green().bold(),
            num_transfers
        );

        Ok(())
    }

    /// Verify account balances in parallel by querying the Lotus node
    fn verify_balances_parallel(
        &self,
        accounts: Vec<(String, String, u64)>,
        context: &StepContext,
    ) -> Result<(), Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        let container_name = lotus_container_name(run_id);

        // Shared error collection
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];

        for (account_name, address, expected_amount) in accounts {
            let container = container_name.clone();
            let errors_clone = Arc::clone(&errors);

            let handle = thread::spawn(move || {
                let output = Command::new("docker")
                    .args([
                        "exec",
                        &container,
                        "/usr/local/bin/lotus-bins/lotus",
                        "wallet",
                        "balance",
                        &address,
                    ])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        let balance_str = String::from_utf8_lossy(&out.stdout);
                        let balance_str = balance_str.trim();

                        // Parse balance (format: "XXX FIL")
                        if let Some(balance_fil) = balance_str.strip_suffix(" FIL") {
                            match balance_fil.trim().parse::<f64>() {
                                Ok(balance) => {
                                    let expected = expected_amount as f64;
                                    if balance >= expected {
                                        println!(
                                            "      {} {}: {} FIL (expected: {} FIL)",
                                            "✓".green(),
                                            account_name,
                                            balance,
                                            expected
                                        );
                                    } else {
                                        let error_msg = format!(
                                            "{}: Insufficient balance. Expected at least {} FIL, got {} FIL",
                                            account_name, expected, balance
                                        );
                                        eprintln!("      ✗ {}", error_msg.clone().red());
                                        errors_clone.lock().unwrap().push(error_msg);
                                    }
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "{}: Failed to parse balance '{}': {}",
                                        account_name, balance_fil, e
                                    );
                                    eprintln!("      ✗ {}", error_msg.clone().red());
                                    errors_clone.lock().unwrap().push(error_msg);
                                }
                            }
                        } else {
                            let error_msg = format!(
                                "{}: Unexpected balance format: {}",
                                account_name, balance_str
                            );
                            eprintln!("      ✗ {}", error_msg.clone().red());
                            errors_clone.lock().unwrap().push(error_msg);
                        }
                    }
                    Ok(out) => {
                        let error_msg = format!(
                            "{}: Failed to check balance: {}",
                            account_name,
                            String::from_utf8_lossy(&out.stderr)
                        );
                        eprintln!("      ✗ {}", error_msg.clone().red());
                        errors_clone.lock().unwrap().push(error_msg);
                    }
                    Err(e) => {
                        let error_msg =
                            format!("{}: Failed to execute balance check: {}", account_name, e);
                        eprintln!("      ✗ {}", error_msg.clone().red());
                        errors_clone.lock().unwrap().push(error_msg);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all balance checks to complete
        for handle in handles {
            handle
                .join()
                .map_err(|_| "Thread panicked during balance verification")?;
        }

        // Check if any errors occurred
        let errors_vec = errors.lock().unwrap();
        if !errors_vec.is_empty() {
            let combined_error = errors_vec.join("\n");
            return Err(format!("Balance verification failed:\n{}", combined_error).into());
        }

        Ok(())
    }
}

impl Step for ETHAccFundingStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Fund Ethereum Accounts"
    }

    fn pre_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
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
    fn execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        if self.check_existing_funding(context)? {
            return Ok(());
        }

        let volumes_dir = crate::paths::foc_localnet_docker_volumes();
        self.perform_account_funding(context, &volumes_dir)?;
        Ok(())
    }

    /// Perform post-execution verification for account funding
    fn post_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying account funding...");

        // First, verify all addresses are in context
        let mut accounts_to_verify = Vec::new();
        for (account_name, expected_amount) in FEVM_ACCOUNTS_PREFUNDED.iter() {
            let address_key = format!("{}_address", account_name.to_lowercase());
            let eth_key = format!("{}_eth_address", account_name.to_lowercase());

            let addr = context
                .get(&address_key)
                .ok_or(format!("Missing address for: {}", account_name))?;
            let eth_addr = context
                .get(&eth_key)
                .ok_or(format!("Missing ETH address for: {}", account_name))?;

            println!(
                "      {} {}: {} (ETH: {})",
                "✓".green(),
                account_name,
                addr,
                eth_addr
            );

            accounts_to_verify.push((account_name.to_string(), addr.to_string(), *expected_amount));
        }

        // Verify balances in parallel
        println!("\n    Verifying account balances with Lotus node...");
        self.verify_balances_parallel(accounts_to_verify, context)?;

        println!(
            "\n    {} Account funding step completed!",
            "✓".green().bold()
        );
        println!("      All Ethereum accounts are funded and ready for contract deployment.");

        Ok(())
    }
}
