//! Private key management for Multicall3 deployment.
//!
//! This module handles the retrieval and processing of private keys
//! needed for contract deployment.

use std::error::Error;

/// Get the private key for the deployer from addresses.json
pub fn get_deployer_private_key(
    _multicall3_deployer_address: &str,
) -> Result<String, Box<dyn Error>> {
    use crate::commands::init::keys::load_keys;

    // Load keys from addresses.json
    let keys = load_keys()?;

    // Find the DEPLOYER_MULTICALL3 key
    let key_info = keys
        .iter()
        .find(|k| k.name == "DEPLOYER_MULTICALL3")
        .ok_or("DEPLOYER_MULTICALL3 key not found in addresses.json")?;

    // The private key is stored as hex without 0x prefix, add it for Foundry
    let private_key = if key_info.private_key.starts_with("0x") {
        key_info.private_key.clone()
    } else {
        format!("0x{}", key_info.private_key)
    };

    Ok(private_key)
}
