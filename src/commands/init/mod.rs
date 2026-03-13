//! Init command implementation.
//!
//! This module handles comprehensive initialization of foc-devnet including:
//! - Creating all necessary directories
//! - Generating default configuration
//! - Setting up PATH variables in shell configs
//! - Downloading required artifacts
//! - Building and caching Docker images
//!
//! Requires a clean home directory (or one containing only config.toml).
//! Run `foc-devnet clean` first if re-initializing.

pub mod artifacts;
pub mod config;
pub mod directories;
pub mod keys;
pub mod path_setup;
pub mod repositories;

use tracing::info;

/// Options for environment initialization
pub struct InitOptions {
    pub curio_location: Option<String>,
    pub lotus_location: Option<String>,
    pub filecoin_services_location: Option<String>,
    pub yugabyte_url: Option<String>,
    pub yugabyte_archive: Option<String>,
    pub proof_params_dir: Option<String>,
    pub use_random_mnemonic: bool,
    pub no_docker_build: bool,
}

/// Initialize foc-devnet comprehensively.
///
/// The caller is responsible for checking that the home directory is clean
/// before calling this function (see `clean::is_clean_for_init`).
///
/// # Returns
/// Returns `Ok(())` on successful initialization, or an error if any step fails.
pub fn init_environment(options: InitOptions) -> Result<(), Box<dyn std::error::Error>> {
    info!("Initializing foc-devnet environment...");

    // Create all necessary directories
    directories::create_directories()?;

    // Generate default configuration (reuses existing config.toml if present)
    config::generate_default_config(
        options.curio_location.clone(),
        options.lotus_location.clone(),
        options.filecoin_services_location.clone(),
        options.yugabyte_url.clone(),
    )?;

    // Generate keys
    keys::generate_keys(options.use_random_mnemonic)?;

    // Set up PATH variables
    path_setup::setup_path_variables()?;

    // Download code repositories
    repositories::download_code_repositories()?;

    // Download required artifacts and build Docker images (unless skipped)
    if options.no_docker_build {
        info!("Skipping artifact downloads and Docker image builds (--no-docker-build flag set)");
    } else {
        // Download required artifacts (or copy from local paths)
        artifacts::download_artifacts(options.yugabyte_archive, options.proof_params_dir)?;

        // Build and cache Docker images
        crate::docker::build::build_and_cache_docker_images()?;
    }

    info!("✓ Initialization completed successfully");
    info!("You can now start the devnet with 'foc-devnet start'");

    Ok(())
}
