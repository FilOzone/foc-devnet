//! Lotus-Miner setup utilities.
//!
//! This module provides utilities for setting up directories and finding preseal files.

use std::error::Error;
use std::fs;
use std::path::Path;

use crate::paths::foc_devnet_genesis_sectors_lotus_miner;

/// Set up necessary directories for Lotus-Miner
pub fn setup_miner_directories(volumes_dir: &Path) -> Result<(), Box<dyn Error>> {
    // Create lotus-miner data directory in volumes
    let miner_data_dir = volumes_dir.join("lotus-miner-data");
    fs::create_dir_all(&miner_data_dir)?;
    Ok(())
}

/// Find the pre-seal metadata and key files for the Lotus miner (t01000)
pub fn find_preseal_files(run_id: &str) -> Result<(String, String), Box<dyn Error>> {
    let sectors_dir = foc_devnet_genesis_sectors_lotus_miner(run_id);

    let preseal_file = "pre-seal-t01000.json";
    let preseal_key_file = "pre-seal-t01000.key";

    let preseal_path = sectors_dir.join(preseal_file);
    let preseal_key_path = sectors_dir.join(preseal_key_file);

    if !preseal_path.exists() {
        return Err(format!(
            "Pre-seal metadata file not found: {}",
            preseal_path.display()
        )
        .into());
    }

    if !preseal_key_path.exists() {
        return Err(format!(
            "Pre-seal key file not found: {}",
            preseal_key_path.display()
        )
        .into());
    }

    Ok((preseal_file.to_string(), preseal_key_file.to_string()))
}
