//! MockUSDFC token transfer operations.
//!
//! This module provides utilities for transferring MockUSDFC tokens between addresses.

use super::constants::TRANSACTION_CONFIRMATION_WAIT_SECS;
use crossterm::style::Stylize;
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
) -> Result<(), Box<dyn Error>> {
    println!("      Transferring {} MockUSDFC tokens: {}...", amount, description);

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host", // Use host network to access localhost:1234
            "-v",
            &format!("{}:/workspace", crate::paths::project_root()?.join("contracts/MockUSDFC").display()),
            "foc-builder",
            "bash",
            "-c",
            &format!(
                "cd /workspace && cast send {} \
                 --private-key {} \
                 --rpc-url http://localhost:1234/rpc/v1 \
                 'transfer(address,uint256)' {} {} \
                 --gas-limit 10000000",
                token_address, from_private_key, to_eth_address, amount
            ),
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
            &format!("{}:/workspace", crate::paths::project_root()?.join("contracts/MockUSDFC").display()),
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

    let balance = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(balance)
}