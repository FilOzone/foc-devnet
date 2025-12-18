//! Genesis signers management.
//!
//! This module handles adding BLS signer keys to the genesis file.

use crate::commands::start::genesis::constants;
use crate::commands::start::genesis::keys::get_bls_addresses;
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes_cache, foc_localnet_genesis,
    foc_localnet_lotus_keys,
};
use std::process::Command;
use tracing::info;

/// Add signers to the genesis file.
///
/// Runs `lotus-seed genesis set-signers` to add the BLS signer keys
/// with the configured threshold.
pub fn add_signers_to_genesis(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("  🔑 Adding signers to genesis...");

    let genesis_dir = foc_localnet_genesis(run_id);
    let keys_dir = foc_localnet_lotus_keys(run_id);

    // Get signer BLS addresses
    let addresses = get_bls_addresses(
        "BLS_SIGNER",
        constants::NUM_SIGNER_KEYS
            .try_into()
            .expect("cannot cast NUM_SIGNER_KEYS into usize"),
        run_id,
    )?;

    if addresses.len() != constants::NUM_SIGNER_KEYS as usize {
        return Err(format!(
            "Expected {} BLS signer addresses, found {}",
            constants::NUM_SIGNER_KEYS,
            addresses.len()
        )
        .into());
    }
    info!("    Using signers:");
    for (i, addr) in addresses.iter().enumerate() {
        info!("      Key {}: {}", i + 1, addr);
    }

    // Run lotus-seed genesis set-signers in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes_cache().join("foc-builder");

    // Build docker args with network environment variables
    let mut docker_args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "--name".to_string(),
        format!("foc-{}-genesis-signers", run_id),
    ];

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

    info!("  ✓ Signers added successfully");
    Ok(())
}
