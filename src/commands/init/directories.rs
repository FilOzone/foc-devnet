//! Directory creation utilities for foc-devnet initialization.
//!
//! This module handles the creation of all necessary directories required
//! for foc-devnet to function properly.

use std::fs;
use tracing::info;

use crate::paths::{
    foc_devnet_artifacts, foc_devnet_bin, foc_devnet_code, foc_devnet_docker_volumes,
    foc_devnet_docker_volumes_cache, foc_devnet_docker_volumes_run_specific_root, foc_devnet_home,
    foc_devnet_keys, foc_devnet_runs, foc_devnet_state,
};

/// Create all necessary directories for foc-devnet.
///
/// # Returns
/// Returns `Ok(())` if all directories are created successfully, or an error if creation fails.
pub fn create_directories() -> Result<(), Box<dyn std::error::Error>> {
    info!("Creating necessary directories...");

    let directories = vec![
        foc_devnet_home(),
        foc_devnet_runs(),
        foc_devnet_bin(),
        foc_devnet_state(),
        foc_devnet_keys(),
        foc_devnet_code(),
        foc_devnet_artifacts(),
        foc_devnet_docker_volumes(),
        foc_devnet_docker_volumes_cache(),
        foc_devnet_docker_volumes_run_specific_root(),
    ];

    for dir in directories {
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
            info!("Created: {}", dir.display());
        } else {
            info!("Exists : {}", dir.display());
        }
    }

    Ok(())
}
