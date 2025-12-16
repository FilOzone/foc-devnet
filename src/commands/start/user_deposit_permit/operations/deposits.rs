//! USDFC deposit operations for FilecoinPay.

use super::super::constants::DEPOSIT_AMOUNT_TOKENS;
use super::utils::wait_for_confirmation;
use crate::constants::BUILDER_CONTAINER;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

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
    lotus_rpc_url: &str,
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
                --rpc-url {} \
                --private-key {} \
                --gas-limit 10000000000"#,
                filecoin_pay_address,
                usdfc_address,
                user_eth_address,
                deposit_amount_wei,
                lotus_rpc_url,
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
    lotus_rpc_url: &str,
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
                r#"cast call {} "getAccountInfoIfSettled(address,address)" {} {} --rpc-url {}"#,
                filecoin_pay_address, usdfc_address, user_eth_address, lotus_rpc_url
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
        let balance_decimal = parse_current_funds(current_funds_hex)?;
        return Ok(balance_decimal.to_string());
    }

    let lines: Vec<&str> = output_trim.lines().collect();
    if lines.len() >= 2 {
        // Get currentFunds (second line)
        let current_funds_hex = lines[1].trim();
        let balance_decimal = parse_current_funds(current_funds_hex)?;
        return Ok(balance_decimal.to_string());
    }

    // Handle single hex blob
    if output_trim.starts_with("0x") && output_trim.len() == 2 + 64 * 4 {
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

fn parse_current_funds(hex: &str) -> Result<u128, Box<dyn Error>> {
    u128::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("Failed to parse currentFunds hex: {}", e).into())
}
