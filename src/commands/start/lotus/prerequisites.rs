//! Prerequisites checking for Lotus daemon startup.
//!
//! This module contains functions that verify all prerequisites are met
//! before starting the Lotus daemon container.

use super::super::genesis::constants::GENESIS_FILE;
use crate::paths::{
    foc_localnet_bin, foc_localnet_genesis, foc_localnet_genesis_sectors,
    foc_localnet_proof_parameters,
};
use std::error::Error;
use tracing::info;

const IMAGE_NAME: &str = "foc-lotus";

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
    info!("    ✓ Docker image '{}' found", IMAGE_NAME);

    // Verify lotus binary exists
    let lotus_bin = foc_localnet_bin().join("lotus");
    if !lotus_bin.exists() {
        return Err("Lotus binary not found. Please run 'foc-localnet build lotus' first.".into());
    }

    info!("    ✓ Lotus binary found");
    Ok(())
}

/// Check that genesis file, proof parameters, and sectors exist
pub fn check_genesis_and_params(run_id: &str) -> Result<(), Box<dyn Error>> {
    // Verify genesis file exists
    let genesis_file = verify_genesis_file(run_id)?;
    info!("    ✓ Genesis file found at {}", genesis_file.display());

    // Verify proof parameters exist
    let params_dir = foc_localnet_proof_parameters();
    if !params_dir.exists() || params_dir.read_dir()?.next().is_none() {
        return Err(
            "Filecoin proof parameters not found. They should have been downloaded during genesis preparation.".into(),
        );
    }

    info!("    ✓ Proof parameters found");

    // Verify pre-sealed sectors exist
    let sectors_dir = foc_localnet_genesis_sectors(run_id);
    if !sectors_dir.exists() || sectors_dir.read_dir()?.next().is_none() {
        return Err(
            "Pre-sealed sectors not found. They should have been created during genesis preparation.".into(),
        );
    }

    info!("    ✓ Pre-sealed sectors found");
    Ok(())
}
