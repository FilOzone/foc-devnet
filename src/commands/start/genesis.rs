//! Genesis preparation module for foc-localnet.
//!
//! This module handles one-time setup tasks required before starting the localnet:
//! - Downloading Filecoin proof parameters
//! - Generating BLS keys for lotus
//! - Pre-sealing sectors for genesis block
//!
//! These operations are performed using the foc-builder container and their
//! outputs are cached for reuse across localnet restarts.

use super::docker_utils::load_image_from_tar;
use crate::paths::{
    CONTAINER_FILECOIN_PROOF_PARAMS_PATH, foc_localnet_bin, foc_localnet_docker_volumes,
    foc_localnet_genesis, foc_localnet_genesis_sectors, foc_localnet_lotus_keys,
    foc_localnet_proof_parameters,
};
use crossterm::style::Stylize;
use std::fs;
use std::path::Path;
use std::process::Command;

const BUILDER_IMAGE: &str = "foc-builder";
const SECTOR_SIZE: &str = "2KiB";
const NUM_SECTORS: u32 = 2;
const PROOF_PARAMS_SECTOR_SIZE: &str = "2048";

/// Ensure all genesis prerequisites are prepared.
///
/// This function checks and prepares:
/// 1. Filecoin proof parameters
/// 2. BLS keys (2x)
/// 3. Pre-sealed sectors
///
/// # Returns
/// Returns `Ok(())` if all prerequisites are ready, or an error if preparation fails.
pub fn ensure_genesis_prerequisites() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Checking genesis prerequisites...".blue().bold());

    // Load the builder image from tar file (needed for all genesis operations)
    load_image_from_tar(BUILDER_IMAGE, "Builder")?;
    println!();

    // Ensure proof parameters are downloaded
    ensure_proof_parameters()?;

    // Ensure BLS keys are generated
    ensure_bls_keys()?;

    // Ensure sectors are pre-sealed
    ensure_presealed_sectors()?;

    println!("{}", "✓ All genesis prerequisites are ready".green().bold());
    Ok(())
}

/// Ensure Filecoin proof parameters are downloaded.
///
/// Parameters are downloaded once and cached in ~/.foc-localnet/artifacts/filecoin-proof-parameters/
/// This directory is mounted into lotus containers at /var/tmp/filecoin-proof-parameters/
fn ensure_proof_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = foc_localnet_proof_parameters();

    // Check if parameters already exist
    if params_dir.exists() && params_dir.read_dir()?.next().is_some() {
        println!(
            "  {} Proof parameters already exist at {}",
            "✓".green(),
            params_dir.display()
        );
        return Ok(());
    }

    println!(
        "  {} Downloading proof parameters (this may take a while)...",
        "⬇".cyan()
    );

    // Create the directory
    fs::create_dir_all(&params_dir)?;

    // Run lotus fetch-params in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/output", bin_dir.display()),
            "-v",
            &format!(
                "{}:/root/.cargo",
                builder_volumes_dir.join("cargo").display()
            ),
            "-v",
            &format!(
                "{}:{}",
                params_dir.display(),
                CONTAINER_FILECOIN_PROOF_PARAMS_PATH
            ),
            BUILDER_IMAGE,
            "/bin/bash",
            "-c",
            &format!("/output/lotus fetch-params {}", PROOF_PARAMS_SECTOR_SIZE),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to download proof parameters: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Proof parameters downloaded successfully", "✓".green());
    Ok(())
}

/// Ensure BLS keys are generated for lotus.
///
/// Generates 2 BLS keys using lotus-shed and stores them in separate directories.
fn ensure_bls_keys() -> Result<(), Box<dyn std::error::Error>> {
    let keys_dir = foc_localnet_lotus_keys();

    for i in 1..=2 {
        let key_dir = keys_dir.join(format!("key-{}", i));
        ensure_bls_key(&key_dir, i)?;
    }

    Ok(())
}

/// Ensure a single BLS key exists in the specified directory.
fn ensure_bls_key(key_dir: &Path, key_num: u32) -> Result<(), Box<dyn std::error::Error>> {
    // Check if key already exists
    if key_dir.exists() && key_dir.read_dir()?.next().is_some() {
        println!(
            "  {} BLS key {} already exists at {}",
            "✓".green(),
            key_num,
            key_dir.display()
        );
        return Ok(());
    }

    println!("  {} Generating BLS key {}...", "🔑".cyan(), key_num);

    // Create the directory
    fs::create_dir_all(key_dir)?;

    // Generate key using lotus-shed in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/output", bin_dir.display()),
            "-v",
            &format!(
                "{}:/root/.cargo",
                builder_volumes_dir.join("cargo").display()
            ),
            "-v",
            &format!("{}:/keys", key_dir.display()),
            BUILDER_IMAGE,
            "/bin/bash",
            "-c",
            "cd /keys && /output/lotus-shed keyinfo new bls",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to generate BLS key {}: {}",
            key_num,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!(
        "  {} BLS key {} generated successfully",
        "✓".green(),
        key_num
    );
    Ok(())
}

/// Ensure sectors are pre-sealed for genesis.
///
/// Pre-seals 2 sectors using lotus-seed and stores them in the genesis-sectors directory.
fn ensure_presealed_sectors() -> Result<(), Box<dyn std::error::Error>> {
    let sectors_dir = foc_localnet_genesis_sectors();
    let genesis_dir = foc_localnet_genesis();

    // Check if sectors already exist
    if sectors_dir.exists() && sectors_dir.read_dir()?.next().is_some() {
        println!(
            "  {} Pre-sealed sectors already exist at {}",
            "✓".green(),
            sectors_dir.display()
        );
        return Ok(());
    }

    println!(
        "  {} Pre-sealing {} sectors (size: {})...",
        "⚙".cyan(),
        NUM_SECTORS,
        SECTOR_SIZE
    );

    // Create the directories
    fs::create_dir_all(&sectors_dir)?;
    fs::create_dir_all(&genesis_dir)?;

    // Pre-seal sectors using lotus-seed in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/output", bin_dir.display()),
            "-v",
            &format!(
                "{}:/root/.cargo",
                builder_volumes_dir.join("cargo").display()
            ),
            "-v",
            &format!("{}:/sectors", sectors_dir.display()),
            "-v",
            &format!("{}:/genesis", genesis_dir.display()),
            BUILDER_IMAGE,
            "/bin/bash",
            "-c",
            &format!(
                "cd /sectors && /output/lotus-seed pre-seal --sector-size {} --num-sectors {}",
                SECTOR_SIZE, NUM_SECTORS
            ),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to pre-seal sectors: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Sectors pre-sealed successfully", "✓".green());
    Ok(())
}
