//! FIL transfer and funding operations.
//!
//! This module provides utilities for transferring FIL between addresses
//! and managing the funding process.

use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::commands::start::eth_acc_funding::constants::TRANSACTION_CONFIRMATION_WAIT_SECS;

/// Transfer FIL from one address to another
pub fn transfer_fil(
    from: &str,
    to: &str,
    amount: u64,
    description: &str,
) -> Result<(), Box<dyn Error>> {
    println!("      Transferring {} FIL: {}...", amount, description);

    let output = Command::new("docker")
        .args([
            "exec",
            "foc-lotus",
            "/usr/local/bin/lotus-bins/lotus",
            "send",
            "--from",
            from,
            to,
            amount.to_string().as_str(),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to transfer FIL: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("\r      Transferred {} FIL: {}...", amount, description.dark_green().bold());

    // Wait for transaction to be included in a block and address to be activated
    // F4 addresses need time to be activated on-chain
    println!("      Waiting for transaction confirmation and address activation...");
    thread::sleep(Duration::from_secs(TRANSACTION_CONFIRMATION_WAIT_SECS));

    Ok(())
}
