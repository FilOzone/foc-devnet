//! Embedded assets for foc-devnet.
//!
//! This module contains all external files embedded into the binary
//! using include_bytes! to make the binary self-contained.

use flate2::read::GzDecoder;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tar::Archive;

// Dockerfiles
pub static DOCKERFILE_BUILDER: &[u8] = include_bytes!("../docker/builder/Dockerfile");
pub static DOCKERFILE_CURIO: &[u8] = include_bytes!("../docker/curio/Dockerfile");
pub static DOCKERFILE_LOTUS: &[u8] = include_bytes!("../docker/lotus/Dockerfile");
pub static DOCKERFILE_LOTUS_MINER: &[u8] = include_bytes!("../docker/lotus-miner/Dockerfile");
pub static DOCKERFILE_YUGABYTE: &[u8] = include_bytes!("../docker/yugabyte/Dockerfile");

// Volumes maps
pub static BUILDER_VOLUMES_MAP: &[u8] = include_bytes!("../docker/builder/volumes_map.toml");
pub static CURIO_VOLUMES_MAP: &[u8] = include_bytes!("../docker/curio/volumes_map.toml");
pub static LOTUS_MINER_VOLUMES_MAP: &[u8] =
    include_bytes!("../docker/lotus-miner/volumes_map.toml");
pub static LOTUS_VOLUMES_MAP: &[u8] = include_bytes!("../docker/lotus/volumes_map.toml");
pub static YUGABYTE_VOLUMES_MAP: &[u8] = include_bytes!("../docker/yugabyte/volumes_map.toml");

// MockUSDFC Foundry Project (as tar.gz archive)
pub static MOCKUSDFC_ARCHIVE: &[u8] = include_bytes!("../artifacts/MockUSDFC.tar.gz");

/// Extract the embedded MockUSDFC Foundry project to a target directory
///
/// This function extracts the embedded MockUSDFC tar.gz archive to the specified
/// target directory, creating the proper directory structure.
///
/// # Arguments
///
/// * `target_dir` - The directory where the MockUSDFC project should be extracted
///
/// # Returns
///
/// Returns `Ok(())` if extraction succeeds, or an error if extraction fails
pub fn extract_mockusdfc_project(target_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    // Create target directory if it doesn't exist
    fs::create_dir_all(target_dir)?;

    // Decompress and extract the tar.gz archive
    let tar_gz = GzDecoder::new(MOCKUSDFC_ARCHIVE);
    let mut archive = Archive::new(tar_gz);

    // Extract all files to the target directory
    archive.unpack(target_dir)?;

    Ok(())
}

/// Get a Dockerfile by name
pub fn get_dockerfile(name: &str) -> Option<&'static [u8]> {
    match name {
        "builder" => Some(DOCKERFILE_BUILDER),
        "curio" => Some(DOCKERFILE_CURIO),
        "lotus" => Some(DOCKERFILE_LOTUS),
        "lotus-miner" => Some(DOCKERFILE_LOTUS_MINER),
        "yugabyte" => Some(DOCKERFILE_YUGABYTE),
        _ => None,
    }
}

/// Get a volumes map by name
pub fn get_volumes_map(name: &str) -> Option<&'static [u8]> {
    match name {
        "builder" => Some(BUILDER_VOLUMES_MAP),
        "curio" => Some(CURIO_VOLUMES_MAP),
        "lotus-miner" => Some(LOTUS_MINER_VOLUMES_MAP),
        "lotus" => Some(LOTUS_VOLUMES_MAP),
        "yugabyte" => Some(YUGABYTE_VOLUMES_MAP),
        _ => None,
    }
}
