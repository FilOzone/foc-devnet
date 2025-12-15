//! Key operations for MockUSDFC funding.
//!
//! This module provides utilities for getting user addresses from the state file.

use crate::commands::init::keys::KeyInfo;
use std::error::Error;
use std::fs;

/// Load addresses from the state/addresses.json file
fn load_state_addresses() -> Result<Vec<KeyInfo>, Box<dyn Error>> {
    let state_file = crate::paths::foc_localnet_state().join("addresses.json");
    if !state_file.exists() {
        return Err(format!("State addresses file not found: {}", state_file.display()).into());
    }

    let content = fs::read_to_string(&state_file)?;
    let addresses: Vec<KeyInfo> = serde_json::from_str(&content)?;
    Ok(addresses)
}

/// Get the Ethereum address for a user by name
pub fn get_user_eth_address(user_name: &str) -> Result<String, Box<dyn Error>> {
    let addresses = load_state_addresses()?;
    let user = addresses
        .iter()
        .find(|k| k.name == user_name)
        .ok_or(format!("{} not found in state addresses", user_name))?;

    Ok(user.eth_address
        .as_ref()
        .ok_or(format!("{} does not have an Ethereum address", user_name))?
        .clone())
}

/// Get the Ethereum address for PDP_SP_0
pub fn get_pdp_sp_eth_address() -> Result<String, Box<dyn Error>> {
    get_user_eth_address("PDP_SP_0")
}

/// Get the Ethereum address for USER_0
pub fn get_user_0_eth_address() -> Result<String, Box<dyn Error>> {
    get_user_eth_address("USER_0")
}

/// Get the Ethereum address for USER_1
pub fn get_user_1_eth_address() -> Result<String, Box<dyn Error>> {
    get_user_eth_address("USER_1")
}

/// Get the Ethereum address for USER_2
pub fn get_user_2_eth_address() -> Result<String, Box<dyn Error>> {
    get_user_eth_address("USER_2")
}

/// Get the private key for a user by name from state addresses
pub fn get_user_private_key(user_name: &str) -> Result<String, Box<dyn Error>> {
    let addresses = load_state_addresses()?;
    let user = addresses
        .iter()
        .find(|k| k.name == user_name)
        .ok_or(format!("{} not found in state addresses", user_name))?;

    Ok(user.private_key.clone())
}