//! Contract address storage for MockUSDFC deployment.
//!
//! This module handles saving deployed contract addresses to persistent storage.

use crate::paths::contract_addresses_file;
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;

/// Save contract address to the contract addresses file
pub fn save_contract_address(name: &str, address: &str) -> Result<(), Box<dyn Error>> {
    let file_path = contract_addresses_file();

    // Ensure parent directory exists
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read existing addresses or create new file
    let mut addresses: serde_json::Value = if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Add/update the address
    addresses[name] = serde_json::json!(address);

    // Write back to file
    let content = serde_json::to_string_pretty(&addresses)?;
    fs::write(&file_path, content)?;

    println!(
        "        {} Contract address saved to {}",
        "✓".green(),
        file_path.display()
    );

    Ok(())
}
