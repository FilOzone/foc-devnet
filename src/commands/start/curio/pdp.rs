//! PDP key management for Curio.
//!
//! Handles importing PDP private keys via Curio Web RPC API.

use super::super::step::SetupContext;
use super::constants::{CURIO_WEB_RPC_PORT, PDP_KEY_IMPORT_WAIT_SECS};
use crate::commands::init::keys::KeyInfo;
use std::error::Error;
use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tracing::info;

/// Import PDP private key for a specific PDP SP.
///
/// Uses JSON-RPC to call: CurioWeb.ImportPDPKey
/// Verifies the returned address matches the expected PDP_SP_X address.
pub fn import_pdp_key(context: &SetupContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    info!("Importing PDP private key for PDP SP {}...", sp_index);

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = format!("foc-{}-curio-{}", run_id, sp_index);

    // Get PDP_SP_{sp_index} private key and expected eth address
    let pdp_sp_name = format!("PDP_SP_{}", sp_index);
    let (private_key, expected_eth_address) = get_pdp_sp_credentials(&pdp_sp_name)?;

    // Import key via JSON-RPC
    let returned_address = import_key_via_rpc(&container_name, &private_key)?;

    // Verify the returned address matches expected
    if returned_address.to_lowercase() != expected_eth_address.to_lowercase() {
        return Err(format!(
            "Address mismatch for {}: expected {}, got {}",
            pdp_sp_name, expected_eth_address, returned_address
        )
        .into());
    }

    info!(
        "    PDP key imported and verified for {} ({})",
        pdp_sp_name, returned_address
    );

    Ok(())
}

/// Get PDP SP credentials from addresses.json
fn get_pdp_sp_credentials(pdp_sp_name: &str) -> Result<(String, String), Box<dyn Error>> {
    let state_file = crate::paths::foc_localnet_state().join("addresses.json");
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

/// Import PDP key via Curio Web RPC API
///
/// Returns the Ethereum address from the response
fn import_key_via_rpc(container_name: &str, private_key: &str) -> Result<String, Box<dyn Error>> {
    // Construct JSON-RPC payload
    let payload = format!(
        r#"{{"jsonrpc":"2.0","method":"CurioWeb.ImportPDPKey","params":["{}"],"id":1}}"#,
        private_key
    );

    // Call via curl inside container
    let output = Command::new("docker")
        .args([
            "exec",
            container_name,
            "curl",
            "-s",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload,
            &format!("http://localhost:{}/api/webrpc/v0", CURIO_WEB_RPC_PORT),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to call ImportPDPKey: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Parse JSON response to extract address
    let response = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&response)?;

    // Check for JSON-RPC error
    if let Some(error) = json.get("error") {
        return Err(format!("ImportPDPKey RPC error: {}", error).into());
    }

    // Extract result (the address)
    let address = json
        .get("result")
        .and_then(|r| r.as_str())
        .ok_or("Missing or invalid 'result' in response")?;

    thread::sleep(Duration::from_secs(PDP_KEY_IMPORT_WAIT_SECS));

    Ok(address.to_string())
}
