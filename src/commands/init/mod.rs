//! Init command implementation.
//!
//! This module handles comprehensive initialization of foc-localnet including:
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
pub mod docker;
pub mod path_setup;
pub mod repositories;

/// Initialize foc-localnet comprehensively.
///
/// This command performs complete initialization:
/// 1. Creates all necessary directories
/// 2. Generates default config.toml
/// 3. Sets up PATH variables in shell configs
/// 4. Downloads required artifacts
/// 5. Downloads code repositories
/// 6. Builds and caches Docker images
///
/// # Arguments
/// * `curio_location` - Optional override for Curio repository location
/// * `lotus_location` - Optional override for Lotus repository location
/// * `yugabyte_url` - Optional override for Yugabyte download URL
/// * `force` - Whether to force regeneration of config file
///
/// # Returns
/// Returns `Ok(())` on successful initialization, or an error if any step fails.
pub fn init_environment(
    curio_location: Option<String>,
    lotus_location: Option<String>,
    yugabyte_url: Option<String>,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::style::Stylize;

    println!("{}", "Initializing foc-localnet environment...".bold());

    // Create all necessary directories
    directories::create_directories()?;

    // Generate default configuration
    config::generate_default_config(
        curio_location.clone(),
        lotus_location.clone(),
        yugabyte_url.clone(),
        force,
    )?;

    // Set up PATH variables
    path_setup::setup_path_variables()?;

    // Download code repositories
    repositories::download_code_repositories()?;

    // Download required artifacts
    artifacts::download_artifacts()?;

    // Build and cache Docker images
    docker::build_and_cache_docker_images()?;

    println!("{}", "✓ Initialization completed successfully".green());
    println!(
        "{}",
        "You may need to restart your shell or run 'source ~/.bashrc' (or ~/.zshrc) to use the updated PATH".cyan()
    );
    Ok(())
}
