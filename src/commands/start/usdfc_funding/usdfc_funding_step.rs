//! MockUSDFC Token Distribution step implementation.
//!
//! This module contains the main Step implementation for distributing MockUSDFC tokens
//! to user and service provider addresses.

use super::constants::{token_amount_to_wei, TRANSACTION_CONFIRMATION_WAIT_SECS};
use super::funding_operations::{self, check_mock_usdfc_balance, transfer_mock_usdfc};
use super::key_operations::get_user_private_key;
use crate::commands::start::step::{SetupContext, Step};
use crate::commands::start::usdfc_funding::key_operations::get_user_eth_address;
use crate::constants::USER_ACCOUNT_COUNT;
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use std::error::Error;
use std::path::PathBuf;
use tracing::info;

/// Step for distributing MockUSDFC tokens to users and service providers
pub struct USDFCFundingStep {
    #[allow(dead_code)]
    run_dir: PathBuf,
    active_pdp_sp_count: usize,
}

impl USDFCFundingStep {
    /// Create a new USDFCFundingStep
    pub fn new(run_dir: PathBuf, active_pdp_sp_count: usize) -> Self {
        Self {
            run_dir,
            active_pdp_sp_count,
        }
    }

    /// Check if all recipients already have the required MockUSDFC balance
    fn check_existing_balances(
        &self,
        context: &SetupContext,
        mockusdfc_address: &str,
        lotus_rpc_url: &str,
    ) -> Result<bool, Box<dyn Error>> {
        // Build list of accounts to check (same as what will be transferred)
        let mut accounts_to_check = Vec::new();

        // Add user accounts (base-1 numbering)
        for user_num in 1..=USER_ACCOUNT_COUNT {
            let account_name = format!("USER_{}", user_num);
            accounts_to_check.push((account_name, 100_000u64));
        }

        // Add only active PDP SPs (base-1 numbering)
        for sp_num in 1..=self.active_pdp_sp_count {
            let account_name = format!("PDP_SP_{}", sp_num);
            accounts_to_check.push((account_name, 200_000u64));
        }

        // Check balances for all accounts
        for (account_name, amount_tokens) in accounts_to_check.iter() {
            let eth_address = get_user_eth_address(account_name)?;

            match check_mock_usdfc_balance(context, &eth_address, mockusdfc_address, lotus_rpc_url)
            {
                Ok(balance) => {
                    let expected_wei = token_amount_to_wei(*amount_tokens);
                    if balance < expected_wei {
                        return Ok(false);
                    }
                }
                Err(e) => {
                    return Err(format!("Failed to check {} balance: {}", account_name, e).into());
                }
            }
        }

        Ok(true)
    }

    /// Execute multiple USDFC transfers in parallel
    fn parallel_transfer_usdfc(
        &self,
        deployer_private_key: &str,
        deployer_mockusdfc_eth: &str,
        mockusdfc_address: &str,
        transfers: Vec<(String, String, ethers_core::types::U256, u64)>,
        lotus_rpc_url: &str,
        context: &SetupContext,
    ) -> Result<(), Box<dyn Error>> {
        use std::sync::{Arc, Mutex};
        use std::thread;

        const MAX_CONCURRENT_TRANSFERS: usize = 6;

        let num_transfers = transfers.len();
        info!(
            "Executing {} USDFC transfers (max {} concurrent)...",
            num_transfers, MAX_CONCURRENT_TRANSFERS
        );

        // Shared error collection
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // Process transfers in batches to avoid overwhelming the RPC
        for (batch_idx, batch) in transfers.chunks(MAX_CONCURRENT_TRANSFERS).enumerate() {
            let batch_num = batch_idx + 1;
            let total_batches = num_transfers.div_ceil(MAX_CONCURRENT_TRANSFERS);

            info!(
                "Processing batch {}/{} ({} transfers)...",
                batch_num,
                total_batches,
                batch.len()
            );

            let mut handles = vec![];

            for (batch_transfer_idx, (account_name, eth_address, amount_wei, amount_tokens)) in
                batch.iter().enumerate()
            {
                let errors_clone = Arc::clone(&errors);
                let context_clone = context.clone();
                let account_name = account_name.clone();
                let eth_address = eth_address.clone();
                let amount_wei_str = amount_wei.to_string();
                let amount = *amount_tokens;
                let pk = deployer_private_key.to_string();
                let _from = deployer_mockusdfc_eth.to_string();
                let contract = mockusdfc_address.to_string();
                let rpc = lotus_rpc_url.to_string();

                // Stagger transfers slightly to avoid nonce conflicts
                if batch_transfer_idx > 0 {
                    thread::sleep(std::time::Duration::from_millis(200));
                }

                let handle = thread::spawn(move || {
                    let description = format!("DEPLOYER_MOCKUSDFC → {}", account_name);
                    info!("Transferring {} USDFC: {}...", amount, description);

                    match transfer_mock_usdfc(
                        &funding_operations::USDFCTransferParams {
                            from_private_key: &pk,
                            to_eth_address: &eth_address,
                            amount: &amount_wei_str,
                            token_address: &contract,
                            description: &description,
                            nonce: Some(
                                (batch_idx * MAX_CONCURRENT_TRANSFERS + batch_transfer_idx + 1)
                                    as u64,
                            ),
                            lotus_rpc_url: &rpc,
                        },
                        &context_clone,
                    ) {
                        Ok(_) => {
                            info!("Transferred {} USDFC: {}", amount, description);
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "Failed to transfer {} USDFC to {}: {}",
                                amount, account_name, e
                            );
                            tracing::error!(" {}", error_msg);
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
                    .map_err(|_| "Thread panicked during transfer")?;
            }

            info!("Batch {}/{} completed", batch_num, total_batches);

            // Wait for batch to be mined before starting next batch
            if batch_num < total_batches {
                info!("Waiting for batch to be mined...");
                thread::sleep(std::time::Duration::from_secs(
                    TRANSACTION_CONFIRMATION_WAIT_SECS,
                ));
            }
        }

        // Wait for final batch confirmation
        info!("Waiting for final transaction confirmations...");
        thread::sleep(std::time::Duration::from_secs(
            TRANSACTION_CONFIRMATION_WAIT_SECS,
        ));

        // Check if any errors occurred
        let errors_vec = errors.lock().unwrap();
        if !errors_vec.is_empty() {
            let combined_error = errors_vec.join("\n");
            return Err(format!("One or more transfers failed:\n{}", combined_error).into());
        }

        Ok(())
    }
}

impl Step for USDFCFundingStep {
    fn name(&self) -> &str {
        "MockUSDFC Token Distribution"
    }

    fn pre_execute(&self, _context: &SetupContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        use super::super::lotus_utils::get_lotus_rpc_url;

        info!("- Distributing MockUSDFC tokens...");

        // Check if Lotus is running
        let run_id = context.run_id();
        if !container_is_running(&lotus_container_name(run_id))? {
            return Err("Lotus container is not running".into());
        }
        info!("Lotus is running");

        // Get MockUSDFC contract address
        let mockusdfc_address = context
            .get("mockusdfc_contract_address")
            .ok_or("MockUSDFC contract address not found in context")?;
        info!("MockUSDFC contract deployed");

        // Get deployer address
        let deployer_mockusdfc_eth = context
            .get("deployer_mockusdfc_eth_address")
            .ok_or("DEPLOYER_MOCKUSDFC ETH address not found in context")?;
        info!("DEPLOYER_MOCKUSDFC address available");

        // Get Lotus RPC URL
        let lotus_rpc_url = get_lotus_rpc_url(context)?;

        // Check if distribution is already done
        if self.check_existing_balances(context, &mockusdfc_address, &lotus_rpc_url)? {
            info!("MockUSDFC distribution already completed, skipping...");
            return Ok(());
        }

        // Get deployer private key
        let deployer_private_key = get_user_private_key("DEPLOYER_MOCKUSDFC")?;

        // Build list of transfers
        let mut token_transfers = Vec::new();

        // Add user accounts (base-1 numbering)
        for user_num in 1..=USER_ACCOUNT_COUNT {
            let account_name = format!("USER_{}", user_num);
            let eth_address = get_user_eth_address(&account_name)?;
            let amount_tokens = 100_000u64;
            let amount_wei = token_amount_to_wei(amount_tokens);
            token_transfers.push((account_name, eth_address, amount_wei, amount_tokens));
        }

        // Add only active PDP SPs (base-1 numbering)
        for sp_num in 1..=self.active_pdp_sp_count {
            let account_name = format!("PDP_SP_{}", sp_num);
            let eth_address = get_user_eth_address(&account_name)?;
            let amount_tokens = 200_000u64;
            let amount_wei = token_amount_to_wei(amount_tokens);
            token_transfers.push((account_name, eth_address, amount_wei, amount_tokens));
        }

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

    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        use super::super::lotus_utils::get_lotus_rpc_url;

        info!("Verifying MockUSDFC distribution...");

        // Get Lotus RPC URL
        let lotus_rpc_url = get_lotus_rpc_url(context)?;

        // Build list of accounts to verify
        let mut accounts_to_verify = Vec::new();

        for user_num in 1..=USER_ACCOUNT_COUNT {
            let account_name = format!("USER_{}", user_num);
            accounts_to_verify.push((account_name, 100_000u64));
        }

        for sp_num in 1..=self.active_pdp_sp_count {
            let account_name = format!("PDP_SP_{}", sp_num);
            accounts_to_verify.push((account_name, 200_000u64));
        }

        for (account_name, amount_tokens) in accounts_to_verify.iter() {
            let eth_address = get_user_eth_address(account_name)?;
            let mock_usdfc_address = context
                .get("mockusdfc_contract_address")
                .ok_or("MockUSDFC address not found in context")?;

            match check_mock_usdfc_balance(
                context,
                &eth_address,
                &mock_usdfc_address,
                &lotus_rpc_url,
            ) {
                Ok(balance) => {
                    let expected_wei = token_amount_to_wei(*amount_tokens);
                    if balance >= expected_wei {
                        info!("{} balance correct: {} tokens", account_name, amount_tokens);
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

        info!("MockUSDFC distribution step completed!");
        Ok(())
    }
}
