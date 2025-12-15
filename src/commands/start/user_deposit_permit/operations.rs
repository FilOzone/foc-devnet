//! Contract interaction operations for USER_0 deposit and permit.

use super::constants::*;
use crate::constants::BUILDER_CONTAINER;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

/// Convert token amount to wei (18 decimals)
pub fn token_amount_to_wei(amount_tokens: u64) -> String {
    format!("{}000000000000000000", amount_tokens)
}

/// Wait for transaction confirmation
fn wait_for_confirmation() {
    println!(
        "      Waiting {} seconds for transaction confirmation...",
        TRANSACTION_CONFIRMATION_WAIT_SECS
    );
    std::thread::sleep(std::time::Duration::from_secs(
        TRANSACTION_CONFIRMATION_WAIT_SECS,
    ));
}

/// Approve FilecoinPay to spend USDFC tokens
///
/// This must be called before depositWithPermitAndApproveOperator to allow
/// the FilecoinPay contract to transfer tokens from USER_0's account.
///
/// # Arguments
/// * `usdfc_address` - Address of USDFC token contract
/// * `filecoin_pay_address` - Address of FilecoinPay contract (spender)
/// * `user_private_key` - Private key of USER_0
/// * `amount_wei` - Amount to approve in wei
///
/// # Returns
/// Ok(()) if successful, Error otherwise
pub fn approve_usdfc_for_filecoin_pay(
    usdfc_address: &str,
    filecoin_pay_address: &str,
    user_private_key: &str,
    amount_wei: &str,
) -> Result<(), Box<dyn Error>> {
    println!("  Approving FilecoinPay to spend USDFC tokens...");
    println!("    Amount: {} USDFC tokens", DEPOSIT_AMOUNT_TOKENS);

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "bash",
            "-c",
            &format!(
                r#"cast send {} \
                "approve(address,uint256)" \
                {} \
                {} \
                --rpc-url http://localhost:1234/rpc/v1 \
                --private-key {} \
                --gas-limit 10000000000"#,
                usdfc_address, filecoin_pay_address, amount_wei, user_private_key
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to approve USDFC for FilecoinPay: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check if transaction was successful
    if stdout.contains("status               0") || stdout.contains("status               (failed)")
    {
        return Err(
            "USDFC approval transaction failed (status 0). Check transaction logs for details."
                .into(),
        );
    }

    println!("  {} Approval successful", "✓".green());

    // Wait for confirmation
    wait_for_confirmation();

    Ok(())
}

/// Query USDFC allowance for FilecoinPay
///
/// This checks how much USDFC the FilecoinPay contract is allowed to spend
/// from USER_0's account.
///
/// # Arguments
/// * `usdfc_address` - Address of USDFC token contract
/// * `user_eth_address` - Ethereum address of USER_0 (owner)
/// * `filecoin_pay_address` - Address of FilecoinPay contract (spender)
///
/// # Returns
/// Allowance in wei as a string, or error
pub fn query_usdfc_allowance(
    usdfc_address: &str,
    user_eth_address: &str,
    filecoin_pay_address: &str,
) -> Result<String, Box<dyn Error>> {
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "bash",
            "-c",
            &format!(
                r#"cast call {} "allowance(address,address)" {} {} --rpc-url http://localhost:1234/rpc/v1"#,
                usdfc_address, user_eth_address, filecoin_pay_address
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query USDFC allowance: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let allowance_hex = stdout.trim();

    // Convert hex to decimal string
    let allowance_decimal = u128::from_str_radix(allowance_hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to parse allowance hex: {}", e))?;

    Ok(allowance_decimal.to_string())
}

/// Deposit USDFC into FilecoinPay
///
/// This function deposits USDFC tokens into the FilecoinPay contract.
/// Requires prior ERC-20 approval for FilecoinPay to spend USDFC.
///
/// # Arguments
/// * `filecoin_pay_address` - Address of FilecoinPay contract
/// * `usdfc_address` - Address of USDFC token contract
/// * `user_eth_address` - Ethereum address of USER_0 (recipient)
/// * `user_private_key` - Private key of USER_0
/// * `deposit_amount_wei` - Amount to deposit in wei
///
/// # Returns
/// Ok(()) if successful, Error otherwise
pub fn deposit_usdfc_to_filecoin_pay(
    filecoin_pay_address: &str,
    usdfc_address: &str,
    user_eth_address: &str,
    user_private_key: &str,
    deposit_amount_wei: &str,
) -> Result<(), Box<dyn Error>> {
    println!("  Depositing USDFC into FilecoinPay...");
    println!("    Amount: {} USDFC tokens", DEPOSIT_AMOUNT_TOKENS);

    // Call deposit(IERC20 token, address to, uint256 amount)
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "bash",
            "-c",
            &format!(
                r#"cast send {} \
                "deposit(address,address,uint256)" \
                {} \
                {} \
                {} \
                --rpc-url http://localhost:1234/rpc/v1 \
                --private-key {} \
                --gas-limit 10000000000"#,
                filecoin_pay_address,
                usdfc_address,
                user_eth_address,
                deposit_amount_wei,
                user_private_key
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to deposit USDFC: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check if transaction was successful
    if stdout.contains("status               0") || stdout.contains("status               (failed)")
    {
        println!("    Transaction output:\n{}", stdout);
        return Err(
            "USDFC deposit transaction failed (status 0). Check transaction logs for details."
                .into(),
        );
    }

    println!("  {} Deposit successful", "✓".green());

    // Wait for confirmation
    wait_for_confirmation();

    Ok(())
}

/// Approve WarmStorage as operator in FilecoinPay
///
/// This function sets operator approval for WarmStorage to create and modify
/// payment rails with rate and lockup limits.
///
/// # Arguments
/// * `filecoin_pay_address` - Address of FilecoinPay contract
/// * `usdfc_address` - Address of USDFC token contract
/// * `warm_storage_address` - Address of WarmStorage contract (operator)
/// * `user_private_key` - Private key of USER_0
///
/// # Returns
/// Ok(()) if successful, Error otherwise
pub fn set_operator_approval(
    filecoin_pay_address: &str,
    usdfc_address: &str,
    warm_storage_address: &str,
    user_private_key: &str,
) -> Result<(), Box<dyn Error>> {
    println!("  Setting WarmStorage as approved operator...");
    println!("    Rate allowance: {} (max uint256)", RATE_ALLOWANCE);
    println!(
        "    Lockup allowance: {} seconds (30 days)",
        LOCKUP_ALLOWANCE_SECONDS
    );
    println!("    Max lockup period: {} (max uint256)", MAX_ALLOWANCE);

    // Call setOperatorApproval(IERC20 token, address operator, bool approved,
    //                          uint256 rateAllowance, uint256 lockupAllowance, uint256 maxLockupPeriod)
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "bash",
            "-c",
            &format!(
                r#"cast send {} \
                "setOperatorApproval(address,address,bool,uint256,uint256,uint256)" \
                {} \
                {} \
                true \
                {} \
                {} \
                {} \
                --rpc-url http://localhost:1234/rpc/v1 \
                --private-key {} \
                --gas-limit 10000000000"#,
                filecoin_pay_address,
                usdfc_address,
                warm_storage_address,
                RATE_ALLOWANCE,
                LOCKUP_ALLOWANCE_SECONDS,
                MAX_ALLOWANCE,
                user_private_key
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to set operator approval: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check if transaction was successful
    if stdout.contains("status               0") || stdout.contains("status               (failed)")
    {
        println!("    Transaction output:\n{}", stdout);
        return Err(
            "Operator approval transaction failed (status 0). Check transaction logs for details."
                .into(),
        );
    }

    println!("  {} Operator approval successful", "✓".green());

    // Wait for confirmation
    wait_for_confirmation();

    Ok(())
}

/// Query USER_0's USDFC balance in FilecoinPay
///
/// # Arguments
/// * `filecoin_pay_address` - Address of FilecoinPay contract
/// * `user_eth_address` - Ethereum address of USER_0
///
/// # Returns
/// Balance in wei as a string, or error
pub fn query_filecoin_pay_balance(
    filecoin_pay_address: &str,
    user_eth_address: &str,
) -> Result<String, Box<dyn Error>> {
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "bash",
            "-c",
            &format!(
                r#"cast call {} "balanceOf(address)" {} --rpc-url http://localhost:1234/rpc/v1"#,
                filecoin_pay_address, user_eth_address
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query FilecoinPay balance: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let balance_hex = stdout.trim();

    // Convert hex to decimal string
    let balance_decimal = u128::from_str_radix(balance_hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to parse balance hex: {}", e))?;

    Ok(balance_decimal.to_string())
}

/// Query operator allowance for WarmStorage on USER_0's account
///
/// This calls the operatorAllowance(address owner, address operator) function
/// which returns (uint256 rateAllowance, uint256 lockupAllowance, uint256 maxAllowance)
///
/// # Arguments
/// * `filecoin_pay_address` - Address of FilecoinPay contract
/// * `user_eth_address` - Ethereum address of USER_0 (owner)
/// * `warm_storage_address` - Address of WarmStorage contract (operator)
///
/// # Returns
/// Tuple of (rateAllowance, lockupAllowance, maxAllowance) as strings in wei, or error
pub fn query_operator_allowance(
    filecoin_pay_address: &str,
    user_eth_address: &str,
    warm_storage_address: &str,
) -> Result<(String, String, String), Box<dyn Error>> {
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "bash",
            "-c",
            &format!(
                r#"cast call {} "operatorAllowance(address,address)" {} {} --rpc-url http://localhost:1234/rpc/v1"#,
                filecoin_pay_address, user_eth_address, warm_storage_address
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query operator allowance: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the output - cast returns tuple values on separate lines
    let lines: Vec<&str> = stdout.trim().lines().collect();

    if lines.len() < 3 {
        return Err(format!(
            "Unexpected output format from operatorAllowance query. Expected 3 lines, got {}",
            lines.len()
        )
        .into());
    }

    // Convert each hex value to decimal
    let rate_allowance = u128::from_str_radix(lines[0].trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to parse rate allowance: {}", e))?
        .to_string();

    let lockup_allowance = u128::from_str_radix(lines[1].trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to parse lockup allowance: {}", e))?
        .to_string();

    let max_allowance = u128::from_str_radix(lines[2].trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to parse max allowance: {}", e))?
        .to_string();

    Ok((rate_allowance, lockup_allowance, max_allowance))
}
