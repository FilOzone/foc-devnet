//! BLS key management for genesis preparation.
//!
//! This module handles generating and managing BLS keys for lotus,
//! including signer keys and additional pre-funded accounts.

use crate::paths::foc_localnet_lotus_keys;
use crossterm::style::Stylize;
use std::fs;
use std::path::Path;

/// Ensure BLS keys are generated for lotus.
///
/// Generates signer keys and additional pre-funded keys using pre-generated keys from init.
/// - Signer keys (key-1, key-2, ...): Used for multisig signers
/// - Pre-funded keys (prefunded-1, prefunded-2, ...): Additional accounts with balance
pub fn ensure_bls_keys() -> Result<(), Box<dyn std::error::Error>> {
    use crate::commands::init::keys::load_keys;

    let keys_dir = foc_localnet_lotus_keys();
    let all_keys = load_keys()?;

    // Filter BLS keys
    let bls_keys: Vec<_> = all_keys
        .iter()
        .filter(|k| k.name.starts_with("BLS_"))
        .collect();

    if bls_keys.len() < super::constants::NUM_SIGNER_KEYS as usize {
        return Err(format!(
            "Not enough BLS keys found. Expected {} BLS keys from init.",
            super::constants::NUM_SIGNER_KEYS
        )
        .into());
    }

    // Generate signer keys (key-1, key-2)
    for i in 1..=super::constants::NUM_SIGNER_KEYS {
        let key_dir = keys_dir.join(format!("key-{}", i));
        let key_name = format!("BLS_SIGNER_{}", i);
        let key_info = bls_keys
            .iter()
            .find(|k| k.name == key_name)
            .ok_or_else(|| format!("BLS key {} not found", key_name))?;
        ensure_bls_key_from_info(&key_dir, key_info, i, "signer")?;
    }

    // Generate GLOBAL_FIL_FAUCET key (prefunded-1)
    let key_dir = keys_dir.join("prefunded-1");
    let key_name = "GLOBAL_FIL_FAUCET";
    let key_info = all_keys
        .iter()
        .find(|k| k.name == key_name)
        .ok_or_else(|| format!("BLS key {} not found", key_name))?;
    ensure_bls_key_from_info(&key_dir, key_info, 1, "prefunded")?;

    Ok(())
}

/// Ensure a single BLS key exists using pre-generated key info.
fn ensure_bls_key_from_info(
    key_dir: &Path,
    key_info: &crate::commands::init::keys::KeyInfo,
    key_num: u32,
    key_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if key already exists
    if key_dir.exists() && key_dir.read_dir()?.next().is_some() {
        println!(
            "  {} BLS {} key {} already exists at {}",
            "✓".green(),
            key_type,
            key_num,
            key_dir.display()
        );
        return Ok(());
    }

    println!(
        "  {} Using pre-generated BLS {} key {} ({})",
        "🔑".cyan(),
        key_type,
        key_num,
        key_info.name
    );

    // Create the directory
    fs::create_dir_all(key_dir)?;

    // Extract address from filecoin_address (remove t3 prefix)
    let address = key_info
        .filecoin_address
        .as_ref()
        .ok_or("BLS key missing filecoin_address")?
        .strip_prefix("t3")
        .ok_or("Invalid BLS address format")?;

    // Create keyinfo file
    let keyinfo_filename = format!("bls-{}.keyinfo", address);
    let keyinfo_path = key_dir.join(keyinfo_filename);

    // Decode the hex-encoded private key
    let private_key_bytes = hex::decode(&key_info.private_key)
        .map_err(|e| format!("Failed to decode private key hex: {}", e))?;

    // Create Lotus-compatible keyinfo JSON structure
    // Lotus expects: {"Type": "bls", "PrivateKey": <base64-encoded bytes>}
    use base64::{engine::general_purpose, Engine as _};
    let keyinfo_json = serde_json::json!({
        "Type": "bls",
        "PrivateKey": general_purpose::STANDARD.encode(&private_key_bytes)
    });

    // Write the JSON keyinfo file
    let json_str = serde_json::to_string(&keyinfo_json)?;
    fs::write(&keyinfo_path, json_str)?;

    println!(
        "  {} BLS {} key {} created successfully at {}",
        "✓".green(),
        key_type,
        key_num,
        address
    );
    Ok(())
}

/// Extract BLS key addresses from the keyinfo files.
///
/// The lotus-shed tool generates keyinfo files with the address encoded in the filename.
/// This function reads the lotus-keys directory and extracts the addresses.
///
/// # Arguments
/// * `key_prefix` - The prefix for key directories (e.g., "key" for signers, "prefunded" for pre-funded)
/// * `count` - Number of keys to extract
///
/// # Returns
/// Returns a vector of BLS addresses (e.g., "f3abc...xyz")
pub fn get_bls_addresses(
    key_prefix: &str,
    count: u32,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let keys_dir = foc_localnet_lotus_keys();
    let mut addresses = Vec::new();

    for i in 1..=count {
        let key_dir = keys_dir.join(format!("{}-{}", key_prefix, i));

        if !key_dir.exists() {
            return Err(format!("BLS key directory {} does not exist", key_dir.display()).into());
        }

        // Find the keyinfo file (should be only one file matching bls-*.keyinfo)
        let entries: Vec<_> = fs::read_dir(&key_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.starts_with("bls-") && s.ends_with(".keyinfo"))
                    .unwrap_or(false)
            })
            .collect();

        if entries.is_empty() {
            return Err(format!("No BLS keyinfo file found in {}", key_dir.display()).into());
        }

        if entries.len() > 1 {
            return Err(format!("Multiple keyinfo files found in {}", key_dir.display()).into());
        }

        // Extract address from filename: bls-<address>.keyinfo
        let filename = entries[0].file_name();
        let filename_str = filename.to_str().ok_or("Invalid filename encoding")?;

        // Remove "bls-" prefix and ".keyinfo" suffix
        let address = filename_str
            .strip_prefix("bls-")
            .and_then(|s| s.strip_suffix(".keyinfo"))
            .ok_or("Invalid keyinfo filename format")?;

        // Add t3 prefix for testnet addresses
        addresses.push(format!("t3{}", address));
    }

    Ok(addresses)
}
