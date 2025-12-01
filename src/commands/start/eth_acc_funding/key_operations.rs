//! Key import, export, and address operations.
//!
//! This module provides utilities for importing keys into Lotus wallet,
//! creating FEVM addresses, and exporting private keys.

use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::paths::foc_localnet_lotus_keys;

/// Import the GLOBAL_FIL_FAUCET key into Lotus wallet
pub fn import_faucet_key(keyinfo_path: &PathBuf) -> Result<String, Box<dyn Error>> {
    println!("      Importing GLOBAL_FIL_FAUCET key into Lotus wallet...");

    // Read the JSON content from the keyinfo file
    let json_content = fs::read_to_string(keyinfo_path)
        .map_err(|e| format!("Failed to read keyinfo file: {}", e))?;

    // Hex-encode the JSON content (lotus wallet import expects hex-encoded JSON)
    let hex_encoded = hex::encode(json_content);

    // Create a temporary file with the hex-encoded content in the same directory
    // so it's accessible via the mounted volume
    let temp_key_file = keyinfo_path.with_extension("keyinfo.hex");
    fs::write(&temp_key_file, &hex_encoded)
        .map_err(|e| format!("Failed to write hex key file: {}", e))?;

    // Get the container path for the temp file
    let keys_dir = foc_localnet_lotus_keys();
    let relative_path = temp_key_file
        .strip_prefix(&keys_dir)
        .map_err(|_| "Failed to get relative path for hex key file")?;
    let container_path = format!("/keys/{}", relative_path.display());

    let output = Command::new("docker")
        .args([
            "exec",
            "foc-lotus",
            "/usr/local/bin/lotus-bins/lotus",
            "wallet",
            "import",
            &container_path,
        ])
        .output()?;

    // Clean up the temp file
    let _ = fs::remove_file(&temp_key_file);

    if !output.status.success() {
        return Err(format!(
            "Failed to import GLOBAL_FIL_FAUCET key: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let address = String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|line| line.starts_with("imported key"))
        .and_then(|line| line.split_whitespace().nth(2))
        .ok_or("Failed to extract imported address")?
        .to_string();

    println!("      {} Key imported: {}", "✓".green(), address);
    Ok(address)
}

/// Create a new f4 (delegated/Ethereum) address for FEVM operations
pub fn create_fevm_address(name: &str) -> Result<String, Box<dyn Error>> {
    println!("      Creating {} f4 address...", name);

    let output = Command::new("docker")
        .args([
            "exec",
            "foc-lotus",
            "/usr/local/bin/lotus-bins/lotus",
            "wallet",
            "new",
            "delegated",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create {} f4 address: {}",
            name,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let address = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!(
        "      {} {} address created: {}",
        "✓".green(),
        name,
        address
    );
    Ok(address)
}

/// Get the Ethereum address corresponding to an f4 address
pub fn get_eth_address(f4_address: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("docker")
        .args([
            "exec",
            "foc-lotus",
            "/usr/local/bin/lotus-bins/lotus",
            "evm",
            "stat",
            f4_address,
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to get Ethereum address: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let eth_addr = output_str
        .lines()
        .find(|line| line.contains("Eth address:"))
        .and_then(|line| line.split_whitespace().nth(2))
        .ok_or("Failed to extract Ethereum address")?
        .to_string();

    Ok(eth_addr)
}

/// Export private key for an f4 address to use with forge/cast
pub fn export_private_key(f4_address: &str, output_file: &PathBuf) -> Result<(), Box<dyn Error>> {
    println!("      Exporting private key for contract deployment...");

    let output = Command::new("docker")
        .args([
            "exec",
            "foc-lotus",
            "/usr/local/bin/lotus-bins/lotus",
            "wallet",
            "export",
            f4_address,
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to export private key: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Write the keyinfo to a file
    fs::write(output_file, &output.stdout)?;
    println!("      {} Private key exported", "✓".green());

    Ok(())
}