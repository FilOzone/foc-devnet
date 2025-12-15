//! MockUSDFC Token Distribution step implementation.
//!
//! This module contains the main Step implementation for distributing MockUSDFC tokens
//! to user and service provider addresses.

use super::constants::{PDP_SP_TOKEN_AMOUNT, USER_TOKEN_AMOUNT};
use super::funding_operations::{check_mock_usdfc_balance, transfer_mock_usdfc};
use super::key_operations::{
    get_pdp_sp_eth_address, get_user_0_eth_address, get_user_1_eth_address,
    get_user_2_eth_address, get_user_private_key,
};
use crate::commands::start::step::{Step, StepContext};
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;

/// Step for distributing MockUSDFC tokens to users and service providers
pub struct USDFCFundingStep {
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl USDFCFundingStep {
    /// Create a new USDFCFundingStep
    pub fn new(_volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self { logs_dir }
    }

    /// Perform the token distribution process
    fn perform_token_distribution(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Distributing MockUSDFC tokens...");

        // Get MockUSDFC contract address from context
        let mock_usdfc_address = context
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

        // Get USER and PDP_SP Ethereum addresses from addresses.json
        let user_0_eth = get_user_0_eth_address()?;
        let user_1_eth = get_user_1_eth_address()?;
        let user_2_eth = get_user_2_eth_address()?;
        let pdp_sp_0_eth = get_pdp_sp_eth_address()?;

        // Store addresses in context
        context.set("user_0_eth_address", &user_0_eth);
        context.set("user_1_eth_address", &user_1_eth);
        context.set("user_2_eth_address", &user_2_eth);
        context.set("pdp_sp_0_eth_address", &pdp_sp_0_eth);

        // Transfer tokens to users
        transfer_mock_usdfc(
            &deployer_private_key,
            &deployer_mockusdfc_eth,
            &user_0_eth,
            USER_TOKEN_AMOUNT,
            &mock_usdfc_address,
            "DEPLOYER_MOCKUSDFC → USER_0",
        )?;
        context.set("user_0_distributed", "true");

        transfer_mock_usdfc(
            &deployer_private_key,
            &deployer_mockusdfc_eth,
            &user_1_eth,
            USER_TOKEN_AMOUNT,
            &mock_usdfc_address,
            "DEPLOYER_MOCKUSDFC → USER_1",
        )?;
        context.set("user_1_distributed", "true");

        transfer_mock_usdfc(
            &deployer_private_key,
            &deployer_mockusdfc_eth,
            &user_2_eth,
            USER_TOKEN_AMOUNT,
            &mock_usdfc_address,
            "DEPLOYER_MOCKUSDFC → USER_2",
        )?;
        context.set("user_2_distributed", "true");

        // Transfer tokens to PDP_SP_0
        transfer_mock_usdfc(
            &deployer_private_key,
            &deployer_mockusdfc_eth,
            &pdp_sp_0_eth,
            PDP_SP_TOKEN_AMOUNT,
            &mock_usdfc_address,
            "DEPLOYER_MOCKUSDFC → PDP_SP_0",
        )?;
        context.set("pdp_sp_distributed", "true");

        println!(
            "\n    {} MockUSDFC tokens distributed successfully!",
            "✓".green().bold()
        );
        println!("      USER_0: {} tokens ({})", USER_TOKEN_AMOUNT, user_0_eth);
        println!("      USER_1: {} tokens ({})", USER_TOKEN_AMOUNT, user_1_eth);
        println!("      USER_2: {} tokens ({})", USER_TOKEN_AMOUNT, user_2_eth);
        println!("      PDP_SP_0: {} tokens ({})", PDP_SP_TOKEN_AMOUNT, pdp_sp_0_eth);

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
        println!("    Verifying MockUSDFC distribution...");

        // Check if all distribution flags are set
        let required_flags = vec![
            "user_0_distributed",
            "user_1_distributed",
            "user_2_distributed",
            "pdp_sp_distributed",
        ];

        for flag in required_flags {
            if let Some(value) = context.get(flag) {
                println!("      {} {}: {}", "✓".green(), flag, value);
            } else {
                println!("      {} {} not found in context", "✗".red(), flag);
                return Err(format!("Missing distribution flag: {}", flag).into());
            }
        }

        // Verify token balances (optional - can be expensive)
        let mock_usdfc_address = context
            .get("mock_usdfc_address")
            .ok_or("MockUSDFC address not found in context")?;

        let user_0_eth = context
            .get("user_0_eth_address")
            .ok_or("USER_0 ETH address not found in context")?;

        // Check one balance as a sanity check
        match check_mock_usdfc_balance(user_0_eth, mock_usdfc_address) {
            Ok(balance) => {
                println!("      {} USER_0 balance check: {} wei", "✓".green(), balance);
            }
            Err(e) => {
                println!("      {} Balance check failed (non-critical): {}", "⚠".yellow(), e);
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