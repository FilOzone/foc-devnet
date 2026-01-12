//! Init command implementation.
//!
//! This module handles comprehensive initialization of foc-devnet including:
//! - Creating all necessary directories
//! - Generating default configuration
//! - Setting up PATH variables in shell configs
//! - Downloading required artifacts
//! - Building and caching Docker images
//!
//! The initialization process is broken down into logical modules for maintainability.

pub mod artifacts;
pub mod config;
pub mod directories;
pub mod keys;
pub mod path_setup;
pub mod repositories;

use tracing::{info, warn};

/// Clean up previous foc-devnet installation.
///
/// Removes the entire ~/.foc-devnet directory and optionally all foc-* Docker
/// images to ensure a clean slate for initialization.
///
/// # Arguments
/// * `remove_images` - When true, also remove cached foc-* Docker images. This
///   should stay false when callers rely on preloaded images (e.g., CI caches).
///
/// # Returns
/// Returns `Ok(())` if cleanup succeeds, or an error if cleanup fails.
fn cleanup_previous_installation(remove_images: bool) -> Result<(), Box<dyn std::error::Error>> {
    use crate::paths::foc_devnet_home;
    use std::process::Command;

    info!("Cleaning up previous installation...");

    // Remove the entire foc-devnet home directory
    let home_dir = foc_devnet_home();
    if home_dir.exists() {
        info!("Removing {}", home_dir.display());
        std::fs::remove_dir_all(&home_dir)?;
        info!("Removed previous foc-devnet installation");
    } else {
        info!("No previous installation found");
    }

    // Optionally remove foc-devnet Docker images
    if remove_images {
        info!("Removing existing foc-devnet Docker images");
        let output = Command::new("docker")
            .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut removed_count = 0;

            for line in stdout.lines() {
                if line.starts_with("foc-") {
                    // Remove the image
                    let remove_output = Command::new("docker").args(["rmi", line]).output()?;

                    if remove_output.status.success() {
                        removed_count += 1;
                    }
                }
            }

            if removed_count > 0 {
                info!("Removed {} Docker image(s)", removed_count);
            } else {
                info!("No foc-devnet Docker images found");
            }
        } else {
            warn!("Could not list Docker images (Docker may not be running)");
        }
    }

    Ok(())
}

/// Initialize foc-devnet comprehensively.
///
/// This command performs complete initialization:
/// 1. Cleans up previous installation (removes ~/.foc-devnet and docker images)
/// 2. Creates all necessary directories
/// 3. Generates default config.toml
/// 4. Sets up PATH variables in shell configs
/// 5. Downloads required artifacts
/// 6. Downloads code repositories
/// 7. Builds and caches Docker images
///
/// # Arguments
/// * `curio_location` - Optional override for Curio repository location
/// * `lotus_location` - Optional override for Lotus repository location
/// * `filecoin_services_location` - Optional override for Filecoin Services repository location
/// * `yugabyte_url` - Optional override for Yugabyte download URL
/// * `yugabyte_archive` - Optional path to local Yugabyte archive file
/// * `proof_params_dir` - Optional path to local filecoin-proof-params directory
/// * `force` - Whether to force regeneration of config file
/// * `use_random_mnemonic` - Whether to use random mnemonic for key generation
/// * `no_docker_build` - Whether to skip artifact downloads and Docker image builds (use when images are already cached)
///
/// # Returns
/// Returns `Ok(())` on successful initialization, or an error if any step fails.
pub fn init_environment(
    curio_location: Option<String>,
    lotus_location: Option<String>,
    filecoin_services_location: Option<String>,
    synapse_sdk_location: Option<String>,
    yugabyte_url: Option<String>,
    yugabyte_archive: Option<String>,
    proof_params_dir: Option<String>,
    force: bool,
    use_random_mnemonic: bool,
    no_docker_build: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Initializing foc-devnet environment...");

    // Clean up previous installation
    // Preserve cached Docker images when --no-docker-build is used (CI cache path)
    cleanup_previous_installation(!no_docker_build)?;

    // Create all necessary directories
    directories::create_directories()?;

    // Generate default configuration
    config::generate_default_config(
        curio_location.clone(),
        lotus_location.clone(),
        filecoin_services_location.clone(),
        synapse_sdk_location.clone(),
        yugabyte_url.clone(),
        force,
    )?;

    // Generate keys
    keys::generate_keys(use_random_mnemonic)?;

    // Set up PATH variables
    path_setup::setup_path_variables()?;

    // Download code repositories
    repositories::download_code_repositories()?;

    // Download required artifacts and build Docker images (unless skipped)
    if no_docker_build {
        info!("Skipping artifact downloads and Docker image builds (--no-docker-build flag set)");
    } else {
        // Download required artifacts (or copy from local paths)
        artifacts::download_artifacts(yugabyte_archive, proof_params_dir)?;

        // Build and cache Docker images
        crate::docker::build::build_and_cache_docker_images()?;
    }

    info!("✓ Initialization completed successfully");
    info!("You can now start the devnet with 'foc-devnet start'");

    Ok(())
}
