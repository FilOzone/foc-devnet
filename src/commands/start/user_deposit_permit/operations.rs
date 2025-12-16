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
/// Uses getAccountInfoIfSettled which returns (fundedUntilEpoch, currentFunds, availableFunds, currentLockupRate)
///
/// # Arguments
/// * `filecoin_pay_address` - Address of FilecoinPay contract
/// * `usdfc_address` - Address of USDFC token
/// * `user_eth_address` - Ethereum address of USER_0
///
/// # Returns
/// Balance (currentFunds) in wei as a string, or error
pub fn query_filecoin_pay_balance(
    filecoin_pay_address: &str,
    usdfc_address: &str,
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
                r#"cast call {} "getAccountInfoIfSettled(address,address)" {} {} --rpc-url http://localhost:1234/rpc/v1"#,
                filecoin_pay_address, usdfc_address, user_eth_address
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query FilecoinPay account info: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let output_trim = stdout.trim();

    // Cast returns hex values - for tuples, it may return as:
    // 1. Comma-separated (0x..., 0x..., ...)
    // 2. Multi-line (0x...\n0x...\n...)
    // 3. Single hex blob (0x[padded 32 bytes][padded 32 bytes][...])

    if output_trim.contains(',') {
        // Format: 0x..., 0x..., 0x..., 0x...
        let parts: Vec<&str> = output_trim.split(',').map(|s| s.trim()).collect();
        if parts.len() < 2 {
            return Err(format!(
                "Unexpected tuple format. Expected at least 2 values, got {}",
                parts.len()
            )
            .into());
        }
        // Get currentFunds (second value in tuple)
        let current_funds_hex = parts[1].trim();
        let balance_decimal = u128::from_str_radix(current_funds_hex.trim_start_matches("0x"), 16)
            .map_err(|e| format!("Failed to parse currentFunds hex: {}", e))?;
        return Ok(balance_decimal.to_string());
    }

    let lines: Vec<&str> = output_trim.lines().collect();
    if lines.len() >= 2 {
        // Get currentFunds (second line)
        let current_funds_hex = lines[1].trim();
        let balance_decimal = u128::from_str_radix(current_funds_hex.trim_start_matches("0x"), 16)
            .map_err(|e| format!("Failed to parse currentFunds hex: {}", e))?;
        return Ok(balance_decimal.to_string());
    }

    // Handle single hex blob (0x[padded 32 bytes][padded 32 bytes][...])
    if output_trim.starts_with("0x") && output_trim.len() == 2 + 64 * 4 {
        // getAccountInfoIfSettled returns 4 fields, each 32 bytes (64 hex chars)
        // currentFunds is the second field (index 1)
        let hex = &output_trim[2..];
        let current_funds_hex = &hex[64..128];
        let balance_decimal = u128::from_str_radix(current_funds_hex, 16)
            .map_err(|e| format!("Failed to parse currentFunds hex from blob: {}", e))?;
        return Ok(balance_decimal.to_string());
    }

    Err(format!(
        "Could not parse getAccountInfoIfSettled output: {}",
        output_trim
    )
    .into())
}

/// Query operator allowance for WarmStorage on USER_0's account
///
/// This calls the auto-generated getter for the public mapping:
/// mapping(IERC20 token => mapping(address client => mapping(address operator => OperatorApproval)))
///
/// OperatorApproval struct returns: (isApproved, rateAllowance, lockupAllowance, rateUsage, lockupUsage, maxLockupPeriod)
///
/// # Arguments
/// * `filecoin_pay_address` - Address of FilecoinPay contract
/// * `usdfc_address` - Address of USDFC token
/// * `user_eth_address` - Ethereum address of USER_0 (client/owner)
/// * `warm_storage_address` - Address of WarmStorage contract (operator)
///
/// # Returns
/// Tuple of (rateAllowance, lockupAllowance, maxLockupPeriod) as strings in wei, or error
pub fn query_operator_allowance(
    filecoin_pay_address: &str,
    usdfc_address: &str,
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
                r#"cast call {} "operatorApprovals(address,address,address)" {} {} {} --rpc-url http://localhost:1234/rpc/v1"#,
                filecoin_pay_address, usdfc_address, user_eth_address, warm_storage_address
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query operator approval: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let output_trim = stdout.trim();

    // Parse the output - cast may return struct fields as:
    // 1. Comma-separated (0x..., 0x..., ...)
    // 2. Multi-line (0x...\n0x...\n...)
    // 3. Single hex blob (0x[padded 32 bytes][padded 32 bytes][...])

    if output_trim.contains(',') {
        let parts: Vec<&str> = output_trim.split(',').map(|s| s.trim()).collect();
        if parts.len() < 6 {
            return Err(format!(
                "Unexpected tuple format from operatorApprovals. Expected 6 values, got {}",
                parts.len()
            )
            .into());
        }
        // Get rateAllowance (index 1), lockupAllowance (index 2), maxLockupPeriod (index 5)
        let rate_allowance = u128::from_str_radix(parts[1].trim_start_matches("0x"), 16)
            .map_err(|e| format!("Failed to parse rate allowance: {}", e))?
            .to_string();
        let lockup_allowance = u128::from_str_radix(parts[2].trim_start_matches("0x"), 16)
            .map_err(|e| format!("Failed to parse lockup allowance: {}", e))?
            .to_string();
        let max_lockup_period = u128::from_str_radix(parts[5].trim_start_matches("0x"), 16)
            .map_err(|e| format!("Failed to parse max lockup period: {}", e))?
            .to_string();
        return Ok((rate_allowance, lockup_allowance, max_lockup_period));
    }

    let lines: Vec<&str> = output_trim.lines().collect();
    if lines.len() >= 6 {
        // Get rateAllowance (index 1), lockupAllowance (index 2), maxLockupPeriod (index 5)
        let rate_allowance = u128::from_str_radix(lines[1].trim_start_matches("0x"), 16)
            .map_err(|e| format!("Failed to parse rate allowance: {}", e))?
            .to_string();
        let lockup_allowance = u128::from_str_radix(lines[2].trim_start_matches("0x"), 16)
            .map_err(|e| format!("Failed to parse lockup allowance: {}", e))?
            .to_string();
        let max_lockup_period = u128::from_str_radix(lines[5].trim_start_matches("0x"), 16)
            .map_err(|e| format!("Failed to parse max lockup period: {}", e))?
            .to_string();
        return Ok((rate_allowance, lockup_allowance, max_lockup_period));
    }

    // Handle single hex blob (0x[padded 32 bytes][padded 32 bytes][...])
    if output_trim.starts_with("0x") && output_trim.len() == 2 + 64 * 6 {
        // operatorApprovals returns 6 fields, each 32 bytes (64 hex chars)
        // rateAllowance: index 1, lockupAllowance: index 2, maxLockupPeriod: index 5
        let hex = &output_trim[2..];

        // Extract the hex values (without parsing to decimal to avoid overflow)
        let rate_allowance_hex = &hex[64..128];
        let lockup_allowance_hex = &hex[128..192];
        let max_lockup_period_hex = &hex[320..384];

        // Just return the hex strings with "0x" prefix for verification
        // The verification will check these match expected values
        return Ok((
            format!("0x{}", rate_allowance_hex),
            format!("0x{}", lockup_allowance_hex),
            format!("0x{}", max_lockup_period_hex),
        ));
    }

    Err(format!(
        "Could not parse operatorApprovals output. Expected 6 values but got: {}",
        output_trim
    )
    .into())
}
