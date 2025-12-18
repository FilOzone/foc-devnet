//! Key import, export, and address operations.
//!
//! This module provides utilities for importing keys into Lotus wallet,
//! creating FEVM addresses, and exporting private keys.

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tracing::info;

use crate::commands::start::step::SetupContext;
use crate::docker::containers::lotus_container_name;
use crate::paths::foc_localnet_lotus_keys;

/// Import the GLOBAL_FIL_FAUCET key into Lotus wallet
pub fn import_faucet_key(
    keyinfo_path: &PathBuf,
    context: &SetupContext,
) -> Result<String, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = lotus_container_name(run_id);

    info!("      Importing GLOBAL_FIL_FAUCET key into Lotus wallet...");

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
    let keys_dir = foc_localnet_lotus_keys(run_id);
    let relative_path = temp_key_file
        .strip_prefix(&keys_dir)
        .map_err(|_| "Failed to get relative path for hex key file")?;
    let container_path = format!("/keys/{}", relative_path.display());

    let output = Command::new("docker")
        .args([
            "exec",
            &container_name,
            "/usr/local/bin/lotus-bins/lotus",
            "wallet",
            "import",
            &container_path,
        ])
        .output()?;

    // Clean up the temp file
    let _ = fs::remove_file(&temp_key_file);

    // Check if import failed
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // If key already exists, that's fine - just get the existing address
        if stderr.contains("key already exists") {
            info!("      Key already exists in wallet");

            // Extract the address from the error message
            // Error format: "...checking key before put 'wallet-<address>': key already exists"
            let address = stderr
                .split("wallet-")
                .nth(1)
                .and_then(|s| s.split('\'').next())
                .ok_or("Failed to extract existing address from error")?
                .to_string();

            info!("      Using existing key: {}", address);
            return Ok(address);
        }

        // For other errors, fail
        return Err(format!("Failed to import GLOBAL_FIL_FAUCET key: {}", stderr).into());
    }

    // Key was successfully imported
    let address = String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|line| line.starts_with("imported key"))
        .and_then(|line| line.split_whitespace().nth(2))
        .ok_or("Failed to extract imported address")?
        .to_string();

    info!("      Key imported: {}", address);
    Ok(address)
}
