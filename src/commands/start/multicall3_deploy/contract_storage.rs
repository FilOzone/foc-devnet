//! Contract address storage for Multicall3 deployment.
//!
//! This module handles saving deployed contract addresses to persistent storage.

use crate::paths::contract_addresses_file;
use std::error::Error;
use std::fs;
use tracing::info;

/// Save contract address to the contract addresses file
#[allow(dead_code)]
pub fn save_contract_address(
    run_id: &str,
    name: &str,
    address: &str,
) -> Result<(), Box<dyn Error>> {
    let file_path = contract_addresses_file(run_id);

    // Ensure parent directory exists
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read existing addresses or create new file
    let mut addresses: serde_json::Value = if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({
            "contracts": {},
            "foc_contracts": {}
        }))
    } else {
        serde_json::json!({
            "contracts": {},
            "foc_contracts": {}
        })
    };

    // Ensure contracts section exists
    if !addresses["contracts"].is_object() {
        addresses["contracts"] = serde_json::json!({});
    }

    // Add/update the address in contracts section
    addresses["contracts"][name] = serde_json::json!(address);

    // Write back to file
    let content = serde_json::to_string_pretty(&addresses)?;
    fs::write(&file_path, content)?;

    info!("Contract address saved to {}", file_path.display());

    Ok(())
}
