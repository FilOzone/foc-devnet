//! Private key management for Multicall3 deployment.
//!
//! This module handles the retrieval and processing of private keys
//! needed for contract deployment.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Get the private key for the deployer from the exported key file
pub fn get_deployer_private_key(
    volumes_dir: &PathBuf,
    _multicall3_deployer_address: &str,
) -> Result<String, Box<dyn Error>> {
    use base64::{engine::general_purpose, Engine as _};

    // The key was exported by ETHAccFundingStep as deployer-multicall3.key
    // (from account name DEPLOYER_MULTICALL3 with underscores replaced by hyphens)
    let key_file = volumes_dir.join("deployer-multicall3.key");

    if !key_file.exists() {
        return Err(format!(
            "Deployer key file not found at {}. \
             Ensure ETHAccFunding step has completed successfully.",
            key_file.display()
        )
        .into());
    }

    // Read the hex-encoded JSON from the file
    let hex_str = fs::read_to_string(&key_file)?.trim().to_string();

    // Decode from hex to get the JSON string
    let json_bytes =
        hex::decode(&hex_str).map_err(|e| format!("Failed to decode hex output: {}", e))?;

    let keyinfo_str = String::from_utf8(json_bytes)
        .map_err(|e| format!("Failed to convert bytes to string: {}", e))?;

    // Parse the JSON to extract the private key
    let keyinfo: serde_json::Value = serde_json::from_str(&keyinfo_str)
        .map_err(|e| format!("Failed to parse keyinfo JSON: {}", e))?;

    // The private key is in the "PrivateKey" field as a base64 string
    let private_key_b64 = keyinfo
        .get("PrivateKey")
        .and_then(|v| v.as_str())
        .ok_or("PrivateKey field not found in keyinfo")?;

    // Decode from base64
    let private_key_bytes = general_purpose::STANDARD
        .decode(private_key_b64)
        .map_err(|e| format!("Failed to decode private key from base64: {}", e))?;

    // Convert to hex string with 0x prefix
    let private_key_hex = format!("0x{}", hex::encode(&private_key_bytes));

    Ok(private_key_hex)
}
