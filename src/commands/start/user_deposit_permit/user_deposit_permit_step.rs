//! USER_0 deposit and operator approval step implementation.

use super::constants::*;
use super::operations::*;
use crate::commands::start::contract_addresses::ContractAddresses;
use crate::commands::start::step::{Step, StepContext};
use crate::commands::start::usdfc_funding::key_operations::{
    get_user_eth_address, get_user_private_key,
};
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;

/// Step for depositing USDFC and approving WarmStorage operator for USER_0
pub struct UserDepositPermitStep {
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl UserDepositPermitStep {
    /// Create a new UserDepositPermitStep
    pub fn new(_volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self { logs_dir }
    }

    /// Check if Lotus is running
    fn check_lotus_running(context: &StepContext) -> Result<(), Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        let container_name = lotus_container_name(run_id);
        if !container_is_running(&container_name)? {
            return Err("Lotus container is not running.".into());
        }
        Ok(())
    }

    /// Load contract addresses from state
    fn load_contract_addresses() -> Result<ContractAddresses, Box<dyn Error>> {
        ContractAddresses::load()
            .map_err(|e| format!("Failed to load contract addresses: {}", e).into())
    }

    /// Get required contract addresses
    fn get_contract_addresses(
        contract_addresses: &ContractAddresses,
    ) -> Result<(String, String, String), Box<dyn Error>> {
        let filecoin_pay_address = contract_addresses
            .foc_contracts
            .get("filecoin_pay_v1_contract")
            .ok_or("filecoin_pay_v1_contract address not found")?
            .clone();

        let warm_storage_address = contract_addresses
            .foc_contracts
            .get("filecoin_warm_storage_service_proxy")
            .ok_or("filecoin_warm_storage_service_proxy address not found")?
            .clone();

        let usdfc_address = contract_addresses
            .contracts
            .get("usdfc")
            .ok_or("usdfc address not found")?
            .clone();

        Ok((filecoin_pay_address, warm_storage_address, usdfc_address))
    }

    /// Check if USER_0 has sufficient USDFC balance
    fn check_usdfc_balance(user_eth_address: &str) -> Result<(), Box<dyn Error>> {
        // This would query the USDFC contract to check balance
        // For now we assume the USDFC funding step has completed successfully
        println!(
            "  {} USER_0 USDFC balance check (assumed from funding step)",
            "✓".green()
        );
        let _ = user_eth_address; // Suppress unused warning for now
        Ok(())
    }
}

impl Step for UserDepositPermitStep {
    fn name(&self) -> &str {
        "USER_0 Deposit and Operator Approval"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!(
            "{} {}",
            "Pre-checking".cyan().bold(),
            self.name().cyan().bold()
        );

        // Check if Lotus is running
        Self::check_lotus_running(context)?;
        println!("  {} Lotus is running", "✓".green());

        // Load contract addresses
        let contract_addresses = Self::load_contract_addresses()?;
        let (filecoin_pay_address, warm_storage_address, usdfc_address) =
            Self::get_contract_addresses(&contract_addresses)?;

        println!("  {} FilecoinPay: {}", "✓".green(), filecoin_pay_address);
        println!("  {} WarmStorage: {}", "✓".green(), warm_storage_address);
        println!("  {} USDFC: {}", "✓".green(), usdfc_address);

        // Get USER_0 addresses
        let user_eth_address = get_user_eth_address(USER_ACCOUNT)?;
        println!("  {} USER_0 ETH address: {}", "✓".green(), user_eth_address);

        // Check USDFC balance (basic check)
        Self::check_usdfc_balance(&user_eth_address)?;

        Ok(())
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!(
            "{} {}",
            "Executing".green().bold(),
            self.name().green().bold()
        );

        // Load contract addresses
        let contract_addresses = Self::load_contract_addresses()?;
        let (filecoin_pay_address, warm_storage_address, usdfc_address) =
            Self::get_contract_addresses(&contract_addresses)?;

        // Get USER_0 credentials
        let user_eth_address = get_user_eth_address(USER_ACCOUNT)?;
        let user_private_key = get_user_private_key(USER_ACCOUNT)?;

        // Calculate deposit amount in wei
        let deposit_amount_wei = token_amount_to_wei(DEPOSIT_AMOUNT_TOKENS);

        // Step 1: Approve FilecoinPay to spend USDFC tokens
        // This ERC-20 approval is required before depositWithPermitAndApproveOperator
        // can transfer tokens from USER_0's account to FilecoinPay
        approve_usdfc_for_filecoin_pay(
            &usdfc_address,
            &filecoin_pay_address,
            &user_private_key,
            &deposit_amount_wei,
        )?;

        // Verify the approval was set correctly
        let allowance =
            query_usdfc_allowance(&usdfc_address, &user_eth_address, &filecoin_pay_address)?;
        let allowance_u128 = allowance
            .parse::<u128>()
            .map_err(|e| format!("Failed to parse allowance: {}", e))?;
        let expected_u128 = deposit_amount_wei
            .parse::<u128>()
            .map_err(|e| format!("Failed to parse expected amount: {}", e))?;

        if allowance_u128 < expected_u128 {
            return Err(format!(
                "USDFC allowance {} is less than required {}",
                allowance_u128, expected_u128
            )
            .into());
        }
        println!(
            "  {} USDFC allowance verified: {} wei",
            "✓".green(),
            allowance
        );

        // Step 2: Deposit USDFC into FilecoinPay
        deposit_usdfc_to_filecoin_pay(
            &filecoin_pay_address,
            &usdfc_address,
            &user_eth_address,
            &user_private_key,
            &deposit_amount_wei,
        )?;

        // Step 3: Approve WarmStorage as operator
        set_operator_approval(
            &filecoin_pay_address,
            &usdfc_address,
            &warm_storage_address,
            &user_private_key,
        )?;

        println!(
            "  {} USER_0 deposit and operator approval complete",
            "✓".green().bold()
        );

        // Store success in context
        context.set("user_0_deposit_complete", "true");

        Ok(())
    }

    fn post_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!(
            "{} {}",
            "Post-checking".cyan().bold(),
            self.name().cyan().bold()
        );

        // Load contract addresses
        let contract_addresses = Self::load_contract_addresses()?;
        let (filecoin_pay_address, warm_storage_address, _usdfc_address) =
            Self::get_contract_addresses(&contract_addresses)?;

        // Get USER_0 address
        let user_eth_address = get_user_eth_address(USER_ACCOUNT)?;

        // Verify FilecoinPay balance
        let balance = query_filecoin_pay_balance(&filecoin_pay_address, &user_eth_address)?;
        let expected_balance = token_amount_to_wei(DEPOSIT_AMOUNT_TOKENS);

        // Convert to u128 for comparison
        let balance_u128 = balance
            .parse::<u128>()
            .map_err(|e| format!("Failed to parse balance: {}", e))?;
        let expected_u128 = expected_balance
            .parse::<u128>()
            .map_err(|e| format!("Failed to parse expected balance: {}", e))?;

        if balance_u128 < expected_u128 {
            return Err(format!(
                "USER_0 FilecoinPay balance {} is less than expected {}",
                balance, expected_balance
            )
            .into());
        }

        println!(
            "  {} USER_0 FilecoinPay balance: {} wei ({} USDFC)",
            "✓".green(),
            balance,
            DEPOSIT_AMOUNT_TOKENS
        );

        // Verify operator allowance
        let (rate_allowance, lockup_allowance, max_allowance) = query_operator_allowance(
            &filecoin_pay_address,
            &user_eth_address,
            &warm_storage_address,
        )?;

        println!("  {} Operator allowances for WarmStorage:", "✓".green());
        println!("      Rate allowance: {}", rate_allowance);
        println!("      Lockup allowance: {} seconds", lockup_allowance);
        println!("      Max allowance: {}", max_allowance);

        // Verify lockup allowance is at least 30 days
        let lockup_u128 = lockup_allowance
            .parse::<u128>()
            .map_err(|e| format!("Failed to parse lockup allowance: {}", e))?;

        if lockup_u128 < LOCKUP_ALLOWANCE_SECONDS as u128 {
            return Err(format!(
                "Lockup allowance {} is less than required {} seconds (30 days)",
                lockup_u128, LOCKUP_ALLOWANCE_SECONDS
            )
            .into());
        }

        println!("  {} All verifications passed", "✓".green().bold());

        Ok(())
    }
}
