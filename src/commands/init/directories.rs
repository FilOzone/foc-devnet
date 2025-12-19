//! Directory creation utilities for foc-localnet initialization.
//!
//! This module handles the creation of all necessary directories required
//! for foc-localnet to function properly.

use std::fs;
use tracing::info;

use crate::paths::{
    foc_localnet_artifacts, foc_localnet_bin, foc_localnet_code, foc_localnet_docker_volumes,
    foc_localnet_docker_volumes_cache, foc_localnet_docker_volumes_run_specific_root,
    foc_localnet_home, foc_localnet_keys, foc_localnet_runs, foc_localnet_state,
};

/// Create all necessary directories for foc-localnet.
///
/// # Returns
/// Returns `Ok(())` if all directories are created successfully, or an error if creation fails.
pub fn create_directories() -> Result<(), Box<dyn std::error::Error>> {
    info!("Creating necessary directories...");

    let directories = vec![
        foc_localnet_home(),
        foc_localnet_runs(),
        foc_localnet_bin(),
        foc_localnet_state(),
        foc_localnet_keys(),
        foc_localnet_code(),
        foc_localnet_artifacts(),
        foc_localnet_docker_volumes(),
        foc_localnet_docker_volumes_cache(),
        foc_localnet_docker_volumes_run_specific_root(),
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
