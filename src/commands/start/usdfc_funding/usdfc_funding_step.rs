//! MockUSDFC Token Distribution step implementation.
//!
//! This module contains the main Step implementation for distributing MockUSDFC tokens
//! to user and service provider addresses.

use super::constants::{token_amount_to_wei, TRANSACTION_CONFIRMATION_WAIT_SECS};
use super::funding_operations::{check_mock_usdfc_balance, transfer_mock_usdfc};
use super::key_operations::get_user_private_key;
use crate::commands::start::step::{Step, StepContext};
use crate::commands::start::usdfc_funding::key_operations::get_user_eth_address;
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;

/// Step for distributing MockUSDFC tokens to users and service providers
pub struct USDFCFundingStep {
    #[allow(dead_code)]
    logs_dir: PathBuf,
    active_pdp_sp_count: usize,
}

impl USDFCFundingStep {
    /// Create a new USDFCFundingStep
    pub fn new(_volumes_dir: PathBuf, logs_dir: PathBuf, active_pdp_sp_count: usize) -> Self {
        Self {
            logs_dir,
            active_pdp_sp_count,
        }
    }

    /// Perform the token distribution process
    fn perform_token_distribution(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        use super::super::lotus_utils::get_lotus_rpc_url;

        println!("    Distributing MockUSDFC tokens...");

        // Get Lotus RPC URL with dynamic port
        let lotus_rpc_url = get_lotus_rpc_url(context)?;

        // Get MockUSDFC contract address from context
        let mockusdfc_address = context
            .get("mock_usdfc_address")
            .ok_or("MockUSDFC contract address not found in context")?
            .to_string();

        // Get DEPLOYER_MOCKUSDFC Ethereum address from context
        let deployer_mockusdfc_eth = context
            .get("deployer_mockusdfc_eth_address")
            .ok_or("DEPLOYER_MOCKUSDFC Ethereum address not found in context")?
            .to_string();

        // Get DEPLOYER_MOCKUSDFC private key from addresses.json
        let deployer_private_key = get_user_private_key("DEPLOYER_MOCKUSDFC")?;

        // Build list of recipients based on active PDP SP count
        let mut token_transfers = Vec::new();

        // Always fund USER accounts (base-1 numbering)
        for user_num in 1..=3 {
            let account_name = format!("USER_{}", user_num);
            let recepient = get_user_eth_address(&account_name)?;
            token_transfers.push((
                account_name,
                recepient.to_string(),
                token_amount_to_wei(100_000),
                100_000u64,
            ));
        }

        // Fund only active PDP SPs (base-1 numbering)
        println!(
            "      Funding {} active PDP SP(s)...",
            self.active_pdp_sp_count
        );
        for sp_num in 1..=self.active_pdp_sp_count {
            let account_name = format!("PDP_SP_{}", sp_num);
            let recepient = get_user_eth_address(&account_name)?;
            token_transfers.push((
                account_name,
                recepient.to_string(),
                token_amount_to_wei(200_000),
                200_000u64,
            ));
        }

        // Set the number of recipients in context
        context.set(
            "usdfc_tfr_recepient_count",
            &token_transfers.len().to_string(),
        );

        // Execute all USDFC transfers in parallel
        self.parallel_transfer_usdfc(
            &deployer_private_key,
            &deployer_mockusdfc_eth,
            &mockusdfc_address,
            token_transfers,
            &lotus_rpc_url,
            context,
        )?;

        Ok(())
    }

    /// Execute multiple USDFC transfers in parallel with concurrency limit
    ///
    /// This function processes transfers in batches of 4 to avoid overwhelming the RPC endpoint.
    /// If any transfer fails, the entire operation fails after all threads complete.
    fn parallel_transfer_usdfc(
        &self,
        from_private_key: &str,
        from_eth_address: &str,
        token_address: &str,
        transfers: Vec<(String, String, String, u64)>,
        lotus_rpc_url: &str,
        _context: &mut StepContext,
    ) -> Result<(), Box<dyn Error>> {
        use std::sync::{Arc, Mutex};
        use std::thread;

        const MAX_CONCURRENT_TRANSFERS: usize = 6;

        let num_transfers = transfers.len();
        println!(
            "      Executing {} USDFC transfers (max {} concurrent)...",
            num_transfers, MAX_CONCURRENT_TRANSFERS
        );

        // Shared error collection
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // Process transfers in batches
        for (batch_idx, batch) in transfers.chunks(MAX_CONCURRENT_TRANSFERS).enumerate() {
            let batch_num = batch_idx + 1;
            let total_batches =
                (num_transfers + MAX_CONCURRENT_TRANSFERS - 1) / MAX_CONCURRENT_TRANSFERS;

            println!(
                "      Processing batch {}/{} ({} transfers)...",
                batch_num,
                total_batches,
                batch.len()
            );

            let mut handles = vec![];

            for (batch_tfr_idx, (account_name, to_addr, amount_wei, amount_tokens)) in
                batch.iter().enumerate()
            {
                let tfr_idx = batch_idx * MAX_CONCURRENT_TRANSFERS + batch_tfr_idx;
                let from_key = from_private_key.to_string();
                let from_addr = from_eth_address.to_string();
                let token_addr = token_address.to_string();
                let rpc_url = lotus_rpc_url.to_string();
                let errors_clone = Arc::clone(&errors);
                let account_name = account_name.clone();
                let to_addr = to_addr.clone();
                let amount_wei = amount_wei.clone();
                let amount_tokens = *amount_tokens;

                // Stagger container launches to avoid overwhelming Docker
                if batch_tfr_idx > 0 {
                    thread::sleep(std::time::Duration::from_millis(100));
                }

                let handle = thread::spawn(move || {
                    let description = format!("DEPLOYER_MOCKUSDFC → {}", account_name);

                    match transfer_mock_usdfc(
                        &from_key,
                        &from_addr,
                        &to_addr,
                        &amount_wei,
                        &token_addr,
                        &description,
                        Some((tfr_idx + 1).try_into().unwrap()),
                        &rpc_url,
                    ) {
                        Ok(_) => {
                            println!(
                                "      ✓ Transferred {} tokens: {}",
                                amount_tokens,
                                description.dark_green().bold()
                            );
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "Failed to transfer {} tokens to {}: {}",
                                amount_tokens, account_name, e
                            );
                            eprintln!("      ✗ {}", error_msg.clone().red());
                            errors_clone.lock().unwrap().push(error_msg);
                        }
                    }
                });

                handles.push(handle);
            }

            // Wait for this batch to complete
            for handle in handles {
                handle
                    .join()
                    .map_err(|_| "Thread panicked during USDFC transfer")?;
            }

            println!("      Batch {}/{} completed", batch_num, total_batches);
        }

        // Wait for transaction confirmation and address activation
        println!("      Waiting for transaction confirmations...");
        thread::sleep(std::time::Duration::from_secs(
            TRANSACTION_CONFIRMATION_WAIT_SECS * 2,
        ));

        // Check if any errors occurred
        let errors_vec = errors.lock().unwrap();
        if !errors_vec.is_empty() {
            let combined_error = errors_vec.join("\n");
            return Err(format!("One or more USDFC transfers failed:\n{}", combined_error).into());
        }

        println!(
            "      {} All {} transfers completed successfully!",
            "✓".green().bold(),
            num_transfers
        );

        Ok(())
    }
}

impl Step for USDFCFundingStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Distribute MockUSDFC"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        let lotus_name = lotus_container_name(run_id);

        if !container_is_running(&lotus_name)? {
            return Err(format!(
                "Lotus container '{}' is not running. MockUSDFC distribution requires Lotus to be running.",
                lotus_name
            )
            .into());
        }
        println!("    {} Lotus is running", "✓".green());

        // Check if MockUSDFC has been deployed
        if context.get("mock_usdfc_address").is_none() {
            return Err(
                "MockUSDFC contract address not found in context. Ensure MockUSDFC deployment step has been completed."
                    .into(),
            );
        }
        println!("    {} MockUSDFC contract deployed", "✓".green());

        // Check if DEPLOYER_MOCKUSDFC address is available
        if context.get("deployer_mockusdfc_eth_address").is_none() {
            return Err(
                "DEPLOYER_MOCKUSDFC Ethereum address not found in context. Ensure ETHAccFunding step has been completed."
                    .into(),
            );
        }
        println!("    {} DEPLOYER_MOCKUSDFC address available", "✓".green());

        Ok(())
    }

    /// Execute the token distribution process
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        self.perform_token_distribution(context)?;
        Ok(())
    }

    /// Perform post-execution verification for token distribution
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        use super::super::lotus_utils::get_lotus_rpc_url;

        println!("    Verifying MockUSDFC distribution...");

        // Get Lotus RPC URL with dynamic port
        let lotus_rpc_url = get_lotus_rpc_url(context)?;

        // Build list of accounts to verify (same as what was transferred)
        let mut accounts_to_verify = Vec::new();

        // Add user accounts (base-1 numbering)
        for user_num in 1..=3 {
            let account_name = format!("USER_{}", user_num);
            accounts_to_verify.push((account_name, 100_000u64));
        }

        // Add only active PDP SPs (base-1 numbering)
        for sp_num in 1..=self.active_pdp_sp_count {
            let account_name = format!("PDP_SP_{}", sp_num);
            accounts_to_verify.push((account_name, 200_000u64));
        }

        // Verify all accounts for distribution have MockUSDFC tokens as expected
        for (account_name, amount_tokens) in accounts_to_verify.iter() {
            let eth_address = get_user_eth_address(&account_name)?;

            match check_mock_usdfc_balance(
                &eth_address,
                context
                    .get("mock_usdfc_address")
                    .ok_or("MockUSDFC address not found in context")?,
                &lotus_rpc_url,
            ) {
                Ok(balance) => {
                    let expected_wei = token_amount_to_wei(*amount_tokens);
                    if balance == expected_wei {
                        println!(
                            "      {} {} balance correct: {} tokens",
                            "✓".green(),
                            account_name,
                            amount_tokens
                        );
                    } else {
                        return Err(format!(
                            "{} balance incorrect: expected {} wei, found {} wei",
                            account_name, expected_wei, balance
                        )
                        .into());
                    }
                }
                Err(e) => {
                    return Err(format!("Failed to check {} balance: {}", account_name, e).into());
                }
            }
        }

        println!(
            "\n    {} MockUSDFC distribution step completed!",
            "✓".green().bold()
        );
        println!("      All users and service providers have been funded with MockUSDFC tokens.");

        Ok(())
    }
}
