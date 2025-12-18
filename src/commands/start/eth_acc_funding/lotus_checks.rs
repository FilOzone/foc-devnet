//! Lotus connectivity and faucet checks.
//!
//! This module provides utilities for checking Lotus daemon status and retrieving faucet addresses.

use std::error::Error;
use std::fs;
use std::process::Command;

use crate::commands::start::eth_acc_funding::constants::GLOBAL_FIL_FAUCET_KEY;
use crate::commands::start::step::SetupContext;
use crate::docker::containers::lotus_container_name;
use crate::paths::foc_localnet_lotus_keys;

/// Check if Lotus is running and accessible
pub fn check_lotus_running(context: &SetupContext) -> Result<(), Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = lotus_container_name(run_id);

    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            &format!("name=^{}$", container_name),
            "--format",
            "{{.Names}}",
        ])
        .output()?;

    if !String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(&container_name)
    {
        return Err(format!(
            "Lotus container '{}' is not running. ETH account funding requires Lotus to be running with FEVM enabled.",
            container_name
        ).into());
    }

    Ok(())
}

/// Get the global faucet address from the prefunded key
pub fn get_global_faucet_address(run_id: &str) -> Result<String, Box<dyn Error>> {
    let keys_dir = foc_localnet_lotus_keys(run_id);
    let faucet_key_dir = keys_dir.join(GLOBAL_FIL_FAUCET_KEY);

    if !faucet_key_dir.exists() {
        return Err(format!(
            "GLOBAL_FIL_FAUCET key directory not found at {}. \
             Ensure genesis preparation has created this key.",
            faucet_key_dir.display()
        )
        .into());
    }

    // Find the keyinfo file
    let entries: Vec<_> = fs::read_dir(&faucet_key_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|s| s.starts_with("bls-") && s.ends_with(".keyinfo"))
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        return Err(format!("No BLS keyinfo file found in {}", faucet_key_dir.display()).into());
    }

    // Extract address from filename: bls-<address>.keyinfo
    let filename = entries[0].file_name();
    let filename_str = filename.to_str().ok_or("Invalid filename encoding")?;

    let address = filename_str
        .strip_prefix("bls-")
        .and_then(|s| s.strip_suffix(".keyinfo"))
        .ok_or("Invalid keyinfo filename format")?;

    Ok(address.to_string())
}
