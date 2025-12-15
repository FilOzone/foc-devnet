//! MockUSDFC token transfer operations.
//!
//! This module provides utilities for transferring MockUSDFC tokens between addresses.

use super::constants::TRANSACTION_CONFIRMATION_WAIT_SECS;
use crossterm::style::Stylize;
use ethers_core::types::U256;
use hex;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Transfer MockUSDFC tokens from one address to another using cast
pub fn transfer_mock_usdfc(
    from_private_key: &str,
    _from_eth_address: &str,
    to_eth_address: &str,
    amount: &str,
    token_address: &str,
    description: &str,
    nonce: Option<u64>,
) -> Result<(), Box<dyn Error>> {
    println!("      Transferring MockUSDFC tokens: {}...", description);

    let mut cast_cmd = format!(
        "cd /workspace && cast send {} \
         --private-key {} \
         --rpc-url http://localhost:1234/rpc/v1 \
         'transfer(address,uint256)' {} {} \
         --gas-limit 10000000",
        token_address, from_private_key, to_eth_address, amount
    );

    // Add nonce if provided
    if let Some(nonce_val) = nonce {
        cast_cmd.push_str(&format!(" --nonce {}", nonce_val));
    }

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host", // Use host network to access localhost:1234
            "-v",
            &format!(
                "{}:/workspace",
                crate::paths::project_root()?
                    .join("contracts/MockUSDFC")
                    .display()
            ),
            "foc-builder",
            "bash",
            "-c",
            &cast_cmd,
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        println!("        {} Transfer failed", "✗".red());
        if !stdout.is_empty() {
            println!("        Output: {}", stdout);
        }
        if !stderr.is_empty() {
            println!("        Error: {}", stderr);
        }
        return Err(format!("Failed to transfer MockUSDFC tokens: {}", description).into());
    }

    println!("        {} Transfer successful", "✓".green());

    // Wait for transaction confirmation
    println!("      Waiting for transaction confirmation...");
    thread::sleep(Duration::from_secs(TRANSACTION_CONFIRMATION_WAIT_SECS));

    Ok(())
}

/// Check MockUSDFC balance for an address using cast
pub fn check_mock_usdfc_balance(
    eth_address: &str,
    token_address: &str,
) -> Result<String, Box<dyn Error>> {
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            "-v",
            &format!(
                "{}:/workspace",
                crate::paths::project_root()?
                    .join("contracts/MockUSDFC")
                    .display()
            ),
            "foc-builder",
            "bash",
            "-c",
            &format!(
                "cd /workspace && cast call {} \
                 --rpc-url http://localhost:1234/rpc/v1 \
                 'balanceOf(address)' {}",
                token_address, eth_address
            ),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to check balance for {}: {}",
            eth_address,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let balance_hex = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if balance_hex.is_empty() || balance_hex == "0x" {
        return Ok("0".to_string());
    }

    // Remove "0x" prefix if it exists
    let hex_str = balance_hex.strip_prefix("0x").unwrap_or(&balance_hex);

    // Decode hex to bytes
    let bytes = match hex::decode(hex_str) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("Failed to decode hex string: {}: {}", hex_str, e).into()),
    };

    // Convert bytes to U256
    let balance_u256 = U256::from_big_endian(&bytes);

    // Convert U256 to decimal string
    let balance_dec = balance_u256.to_string();

    Ok(balance_dec)
}
