//! Prerequisites checking for Lotus daemon startup.
//!
//! This module contains functions that verify all prerequisites are met
//! before starting the Lotus daemon container.

use super::super::genesis::constants::GENESIS_FILE;
use crate::paths::{foc_devnet_genesis, foc_devnet_genesis_sectors, foc_devnet_proof_parameters};
use std::error::Error;
use tracing::info;

/// Verify that the genesis block file exists
pub fn verify_genesis_file(run_id: &str) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let genesis_dir = foc_devnet_genesis(run_id);
    let genesis_file = genesis_dir.join(GENESIS_FILE);

    if !genesis_file.exists() {
        return Err(
            "Genesis file not found. This should have been created during genesis preparation."
                .into(),
        );
    }

    Ok(genesis_file)
}

/// Check that genesis file, proof parameters, and sectors exist
pub fn check_genesis_and_params(run_id: &str) -> Result<(), Box<dyn Error>> {
    // Verify genesis file exists
    let genesis_file = verify_genesis_file(run_id)?;
    info!("✓ Genesis file found at {}", genesis_file.display());

    // Verify proof parameters exist
    let params_dir = foc_devnet_proof_parameters();
    if !params_dir.exists() || params_dir.read_dir()?.next().is_none() {
        return Err(
            "Filecoin proof parameters not found. They should have been downloaded during genesis preparation.".into(),
        );
    }

    info!("✓ Proof parameters found");

    // Verify pre-sealed sectors exist
    let sectors_dir = foc_devnet_genesis_sectors(run_id);
    if !sectors_dir.exists() || sectors_dir.read_dir()?.next().is_none() {
        return Err(
            "Pre-sealed sectors not found. They should have been created during genesis preparation.".into(),
        );
    }

    info!("✓ Pre-sealed sectors found");
    Ok(())
}
