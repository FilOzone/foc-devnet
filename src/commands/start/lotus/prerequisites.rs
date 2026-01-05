//! Prerequisites checking for Lotus daemon startup.
//!
//! This module contains functions that verify all prerequisites are met
//! before starting the Lotus daemon container.

use super::super::genesis::constants::GENESIS_FILE;
use crate::paths::{
    foc_localnet_bin, foc_localnet_genesis, foc_localnet_genesis_sectors,
    foc_localnet_proof_parameters,
};
use sha2::{Digest, Sha256};
use std::error::Error;
use tracing::info;

const IMAGE_NAME: &str = "foc-lotus";

/// Expected SHA256 hash of the proof parameters directory
const EXPECTED_PROOF_PARAMS_SHA256: &str =
    "1d2746e3c92f96608adfa10849004b5654104082c3bba5a2e5362e0657741527";

/// Compute the SHA256 hash of all files in the proof parameters directory
///
/// This function computes a deterministic hash based only on file contents (not paths) by:
/// 1. Finding all regular files: `find params_dir -type f -exec sha256sum {} \;`
/// 2. Extracting only the hash part (first field), removing file paths
/// 3. Sorting the hashes to ensure consistent ordering
/// 4. Computing SHA256 of the concatenated sorted hashes
fn compute_proof_params_hash(params_dir: &std::path::Path) -> Result<String, Box<dyn Error>> {
    use std::process::Command;

    let output = Command::new("find")
        .arg(params_dir)
        .arg("-type")
        .arg("f")
        .arg("-exec")
        .arg("sha256sum")
        .arg("{}")
        .arg(";")
        .output()?;

    if !output.status.success() {
        return Err("Failed to compute file hashes".into());
    }

    let file_hashes = String::from_utf8(output.stdout)?;
    // Extract only the hash part (first field), not the file path
    let mut hashes: Vec<&str> = file_hashes
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    hashes.sort();

    let combined = hashes.join("\n");
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Verify that the genesis block file exists
pub fn verify_genesis_file(run_id: &str) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let genesis_dir = foc_localnet_genesis(run_id);
    let genesis_file = genesis_dir.join(GENESIS_FILE);

    if !genesis_file.exists() {
        return Err(
            "Genesis file not found. This should have been created during genesis preparation."
                .into(),
        );
    }

    Ok(genesis_file)
}

/// Check that required Docker image and Lotus binary exist
pub fn check_image_and_binary() -> Result<(), Box<dyn Error>> {
    // Verify Docker image exists
    if !crate::docker::core::image_exists(IMAGE_NAME).unwrap_or(true) {
        return Err(format!(
            "Docker image '{}' not found. Please run 'foc-localnet init' to build the image.",
            IMAGE_NAME
        )
        .into());
    }
    info!("✓ Docker image '{}' found", IMAGE_NAME);

    // Verify lotus binary exists
    let lotus_bin = foc_localnet_bin().join("lotus");
    if !lotus_bin.exists() {
        return Err("Lotus binary not found. Please run 'foc-localnet build lotus' first.".into());
    }

    info!("✓ Lotus binary found");
    Ok(())
}

/// Check that genesis file, proof parameters, and sectors exist
pub fn check_genesis_and_params(run_id: &str) -> Result<(), Box<dyn Error>> {
    // Verify genesis file exists
    let genesis_file = verify_genesis_file(run_id)?;
    info!("✓ Genesis file found at {}", genesis_file.display());

    // Verify proof parameters exist
    let params_dir = foc_localnet_proof_parameters();
    if !params_dir.exists() || params_dir.read_dir()?.next().is_none() {
        return Err(
            "Filecoin proof parameters not found. They should have been downloaded during genesis preparation.".into(),
        );
    }

    // Verify proof parameters integrity
    let computed_hash = compute_proof_params_hash(&params_dir)?;
    if computed_hash != EXPECTED_PROOF_PARAMS_SHA256 {
        return Err(format!(
            "Filecoin proof parameters integrity check failed. Expected hash: {}, got: {}",
            EXPECTED_PROOF_PARAMS_SHA256, computed_hash
        )
        .into());
    }

    info!("✓ Proof parameters found and verified");

    // Verify pre-sealed sectors exist
    let sectors_dir = foc_localnet_genesis_sectors(run_id);
    if !sectors_dir.exists() || sectors_dir.read_dir()?.next().is_none() {
        return Err(
            "Pre-sealed sectors not found. They should have been created during genesis preparation.".into(),
        );
    }

    info!("✓ Pre-sealed sectors found");
    Ok(())
}
