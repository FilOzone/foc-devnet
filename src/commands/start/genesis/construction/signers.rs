//! Genesis signers management.
//!
//! This module handles adding BLS signer keys to the genesis file.

use crate::commands::start::env_vars::build_network_env_vars;
use crate::commands::start::genesis::constants;
use crate::commands::start::genesis::keys::get_bls_addresses;
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis, foc_localnet_lotus_keys,
};
use crossterm::style::Stylize;
use std::process::Command;

/// Add signers to the genesis file.
///
/// Runs `lotus-seed genesis set-signers` to add the BLS signer keys
/// with the configured threshold.
pub fn add_signers_to_genesis() -> Result<(), Box<dyn std::error::Error>> {
    println!("  {} Adding signers to genesis...", "🔑".cyan());

    let genesis_dir = foc_localnet_genesis();
    let keys_dir = foc_localnet_lotus_keys();

    // Get signer BLS addresses
    let addresses = get_bls_addresses(
        "BLS_SIGNER",
        constants::NUM_SIGNER_KEYS
            .try_into()
            .expect("cannot cast NUM_SIGNER_KEYS into usize"),
    )?;

    if addresses.len() != constants::NUM_SIGNER_KEYS as usize {
        return Err(format!(
            "Expected {} BLS signer addresses, found {}",
            constants::NUM_SIGNER_KEYS,
            addresses.len()
        )
        .into());
    }
    println!("    Using signers:");
    for (i, addr) in addresses.iter().enumerate() {
        println!("      {} {}", format!("Key {}:", i + 1).dim(), addr);
    }

    // Run lotus-seed genesis set-signers in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    // Build docker args with network environment variables
    let mut docker_args = vec!["run".to_string(), "--rm".to_string()];

    // Add network environment variables (required for lotus-seed built with -tags=localnet)
    docker_args.extend(build_network_env_vars());

    // Add volume mounts and command
    docker_args.extend(vec![
        "-v".to_string(),
        format!("{}:/opt/bin", bin_dir.display()),
        "-v".to_string(),
        format!("{}:/home/foc-user/.cargo", builder_volumes_dir.join("cargo").display()),
        "-v".to_string(),
        format!("{}:/genesis", genesis_dir.display()),
        "-v".to_string(),
        format!("{}:/keys", keys_dir.display()),
        "foc-builder".to_string(),
        "/bin/bash".to_string(),
        "-c".to_string(),
        format!(
            "/opt/bin/lotus-seed genesis set-signers --threshold={} --signers {} --signers {} /genesis/{}",
            constants::SIGNERS_THRESHOLD, addresses[0], addresses[1], constants::GENESIS_FILE
        ),
    ]);

    let output = Command::new("docker").args(&docker_args).output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to add signers to genesis: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Signers added successfully", "✓".green());
    Ok(())
}
