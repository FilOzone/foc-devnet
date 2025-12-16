//! USDFC token approval operations for FilecoinPay.

use super::super::constants::DEPOSIT_AMOUNT_TOKENS;
use super::utils::wait_for_confirmation;
use crate::constants::BUILDER_CONTAINER;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

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
