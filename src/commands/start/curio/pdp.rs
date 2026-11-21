//! PDP key management for Curio.
//!
//! Handles importing PDP private keys via Curio Web RPC API.

use super::super::step::SetupContext;
use super::constants::PDP_KEY_IMPORT_WAIT_SECS;
use super::web_rpc::WebRpc;
use crate::commands::init::keys::KeyInfo;
use std::error::Error;
use std::fs;
use std::thread;
use std::time::Duration;
use tracing::info;

/// Import PDP private key for a specific PDP SP.
///
/// Uses JSON-RPC to call: CurioWeb.ImportPDPKey
/// Verifies the returned address matches the expected PDP_SP_X address.
pub fn import_pdp_key(context: &SetupContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    info!("Importing PDP private key for PDP SP {}...", sp_index);

    let rpc = WebRpc::connect(context, sp_index)?;

    // Get PDP_SP_{sp_index} private key and expected eth address
    let pdp_sp_name = format!("PDP_SP_{}", sp_index);
    let (private_key, expected_eth_address) = get_pdp_sp_credentials(&pdp_sp_name)?;

    // Import key via JSON-RPC
    let returned_address = rpc.import_pdp_key(&private_key)?;
    thread::sleep(Duration::from_secs(PDP_KEY_IMPORT_WAIT_SECS));

    // Verify the returned address matches expected
    if returned_address.to_lowercase() != expected_eth_address.to_lowercase() {
        return Err(format!(
            "Address mismatch for {}: expected {}, got {}",
            pdp_sp_name, expected_eth_address, returned_address
        )
        .into());
    }

    info!(
        "PDP key imported and verified for {} ({})",
        pdp_sp_name, returned_address
    );

    Ok(())
}

/// Get PDP SP credentials from addresses.json
fn get_pdp_sp_credentials(pdp_sp_name: &str) -> Result<(String, String), Box<dyn Error>> {
    let state_file = crate::paths::foc_devnet_keys().join("addresses.json");
    if !state_file.exists() {
        return Err(format!("State addresses file not found: {}", state_file.display()).into());
    }

    let content = fs::read_to_string(&state_file)?;
    let addresses: Vec<KeyInfo> = serde_json::from_str(&content)?;

    let pdp_sp = addresses
        .iter()
        .find(|k| k.name == pdp_sp_name)
        .ok_or(format!("{} not found in state addresses", pdp_sp_name))?;

    let eth_address = pdp_sp
        .eth_address
        .as_ref()
        .ok_or(format!("{} does not have an Ethereum address", pdp_sp_name))?
        .clone();

    Ok((pdp_sp.private_key.clone(), eth_address))
}
