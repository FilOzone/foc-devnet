//! Operator approval and management operations for FilecoinPay.

use super::super::constants::{LOCKUP_ALLOWANCE_SECONDS, MAX_ALLOWANCE, RATE_ALLOWANCE};
use super::utils::wait_for_confirmation;
use crate::constants::BUILDER_CONTAINER;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

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
    lotus_rpc_url: &str,
) -> Result<(), Box<dyn Error>> {
    println!("  Setting WarmStorage as approved operator...");
    println!("    Rate allowance: {} (max uint256)", RATE_ALLOWANCE);
    println!(
        "    Lockup allowance: {} seconds (30 days)",
        LOCKUP_ALLOWANCE_SECONDS
    );
    println!("    Max lockup period: {} (max uint256)", MAX_ALLOWANCE);

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
                --rpc-url {} \
                --private-key {} \
                --gas-limit 10000000000"#,
                filecoin_pay_address,
                usdfc_address,
                warm_storage_address,
                RATE_ALLOWANCE,
                LOCKUP_ALLOWANCE_SECONDS,
                MAX_ALLOWANCE,
                lotus_rpc_url,
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
    lotus_rpc_url: &str,
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
                r#"cast call {} "operatorApprovals(address,address,address)" {} {} {} --rpc-url {}"#,
                filecoin_pay_address, usdfc_address, user_eth_address, warm_storage_address, lotus_rpc_url
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query operator approval: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_operator_approval_output(&stdout)
}

fn parse_operator_approval_output(
    output: &str,
) -> Result<(String, String, String), Box<dyn Error>> {
    let output_trim = output.trim();

    // Parse comma-separated format
    if output_trim.contains(',') {
        return parse_comma_separated(output_trim);
    }

    let lines: Vec<&str> = output_trim.lines().collect();
    if lines.len() >= 6 {
        return parse_multiline(&lines);
    }

    // Handle single hex blob
    if output_trim.starts_with("0x") && output_trim.len() == 2 + 64 * 6 {
        return parse_hex_blob(output_trim);
    }

    Err(format!(
        "Could not parse operatorApprovals output. Expected 6 values but got: {}",
        output_trim
    )
    .into())
}

fn parse_comma_separated(output: &str) -> Result<(String, String, String), Box<dyn Error>> {
    let parts: Vec<&str> = output.split(',').map(|s| s.trim()).collect();
    if parts.len() < 6 {
        return Err(format!(
            "Unexpected tuple format from operatorApprovals. Expected 6 values, got {}",
            parts.len()
        )
        .into());
    }
    Ok((
        parse_hex_to_string(parts[1])?,
        parse_hex_to_string(parts[2])?,
        parse_hex_to_string(parts[5])?,
    ))
}

fn parse_multiline(lines: &[&str]) -> Result<(String, String, String), Box<dyn Error>> {
    Ok((
        parse_hex_to_string(lines[1])?,
        parse_hex_to_string(lines[2])?,
        parse_hex_to_string(lines[5])?,
    ))
}

fn parse_hex_blob(output: &str) -> Result<(String, String, String), Box<dyn Error>> {
    let hex = &output[2..];
    let rate_allowance_hex = &hex[64..128];
    let lockup_allowance_hex = &hex[128..192];
    let max_lockup_period_hex = &hex[320..384];

    Ok((
        format!("0x{}", rate_allowance_hex),
        format!("0x{}", lockup_allowance_hex),
        format!("0x{}", max_lockup_period_hex),
    ))
}

fn parse_hex_to_string(hex: &str) -> Result<String, Box<dyn Error>> {
    u128::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map(|v| v.to_string())
        .map_err(|e| format!("Failed to parse hex: {}", e).into())
}
