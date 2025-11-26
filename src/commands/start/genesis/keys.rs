//! BLS key management for genesis preparation.
//!
//! This module handles generating and managing BLS keys for lotus,
//! including signer keys and additional pre-funded accounts.

use crate::paths::{foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_lotus_keys};
use crossterm::style::Stylize;
use std::fs;
use std::path::Path;

/// Ensure BLS keys are generated for lotus.
///
/// Generates signer keys and additional pre-funded keys using lotus-shed.
/// - Signer keys (key-1, key-2, ...): Used for multisig signers
/// - Pre-funded keys (prefunded-1, prefunded-2, ...): Additional accounts with balance
pub fn ensure_bls_keys() -> Result<(), Box<dyn std::error::Error>> {
    let keys_dir = foc_localnet_lotus_keys();

    // Generate signer keys
    for i in 1..=super::constants::NUM_SIGNER_KEYS {
        let key_dir = keys_dir.join(format!("key-{}", i));
        ensure_bls_key(&key_dir, i, "signer")?;
    }

    // Generate additional pre-funded keys (non-signers)
    for i in 1..=super::constants::NUM_PREFUNDED_KEYS {
        let key_dir = keys_dir.join(format!("prefunded-{}", i));
        ensure_bls_key(&key_dir, i, "prefunded")?;
    }

    Ok(())
}

/// Ensure a single BLS key exists in the specified directory.
pub fn ensure_bls_key(
    key_dir: &Path,
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
        "  {} Generating BLS {} key {}...",
        "🔑".cyan(),
        key_type,
        key_num
    );

    // Create the directory
    fs::create_dir_all(key_dir)?;

    // Generate key using lotus-shed in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/opt/bin", bin_dir.display()),
            "-v",
            &format!(
                "{}:/home/foc-user/.cargo",
                builder_volumes_dir.join("cargo").display()
            ),
            "-v",
            &format!("{}:/keys", key_dir.display()),
            "foc-builder",
            "/bin/bash",
            "-c",
            "cd /keys && /opt/bin/lotus-shed keyinfo new bls",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to generate BLS {} key {}: {}",
            key_type,
            key_num,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!(
        "  {} BLS {} key {} generated successfully",
        "✓".green(),
        key_type,
        key_num
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

        addresses.push(address.to_string());
    }

    Ok(addresses)
}
