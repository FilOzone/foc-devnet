//! Lotus-Miner setup utilities.
//!
//! This module provides utilities for setting up directories and finding preseal files.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use crate::paths::foc_localnet_genesis_sectors;

/// Set up necessary directories for Lotus-Miner
pub fn setup_miner_directories(volumes_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    // Create lotus-miner data directory in volumes
    let miner_data_dir = volumes_dir.join("lotus-miner-data");
    fs::create_dir_all(&miner_data_dir)?;
    Ok(())
}

/// Find the pre-seal metadata and key files
pub fn find_preseal_files() -> Result<(String, String), Box<dyn Error>> {
    let sectors_dir = foc_localnet_genesis_sectors();

    let mut preseal_file = None;
    let mut preseal_key_file = None;
    for entry in fs::read_dir(&sectors_dir)? {
        let entry = entry?;
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();

        if path.is_file() {
            if path.extension().map_or(false, |ext| ext == "json")
                && filename.starts_with("pre-seal-")
            {
                preseal_file = Some(filename.clone());
            }
            if path.extension().map_or(false, |ext| ext == "key")
                && filename.starts_with("pre-seal-")
            {
                preseal_key_file = Some(filename);
            }
        }
    }

    let preseal_file =
        preseal_file.ok_or("Pre-seal metadata file not found in sectors directory")?;
    let preseal_key_file =
        preseal_key_file.ok_or("Pre-seal key file not found in sectors directory")?;

    Ok((preseal_file, preseal_key_file))
}
