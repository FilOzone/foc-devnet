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

/// Clean up previous foc-localnet installation.
///
/// This function removes the entire ~/.foc-localnet directory and all
/// previously built Docker images to ensure a clean slate for initialization.
///
/// # Returns
/// Returns `Ok(())` if cleanup succeeds, or an error if cleanup fails.
fn cleanup_previous_installation() -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::style::Stylize;
    use std::process::Command;
    use crate::paths::foc_localnet_home;

    println!("{}", "Cleaning up previous installation...".bold());

    // Remove the entire foc-localnet home directory
    let home_dir = foc_localnet_home();
    if home_dir.exists() {
        println!("  {} Removing {}", "🗑️".yellow(), home_dir.display());
        std::fs::remove_dir_all(&home_dir)?;
        println!("  {} Removed previous foc-localnet installation", "✓".green());
    } else {
        println!("  {} No previous installation found", "✓".green());
    }

    // Remove all foc-localnet Docker images
    println!("  {} Removing existing foc-localnet Docker images", "🗑️".yellow());
    let output = Command::new("docker")
        .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut removed_count = 0;

        for line in stdout.lines() {
            if line.starts_with("foc-") {
                // Remove the image
                let remove_output = Command::new("docker")
                    .args(["rmi", line])
                    .output()?;

                if remove_output.status.success() {
                    removed_count += 1;
                }
            }
        }

        if removed_count > 0 {
            println!("  {} Removed {} Docker image(s)", "✓".green(), removed_count);
        } else {
            println!("  {} No foc-localnet Docker images found", "✓".green());
        }
    } else {
        println!("  {} Could not list Docker images (Docker may not be running)", "⚠".yellow());
    }

    Ok(())
}

/// Initialize foc-localnet comprehensively.
///
/// This command performs complete initialization:
/// 1. Cleans up previous installation (removes ~/.foc-localnet and docker images)
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

    // Clean up previous installation
    cleanup_previous_installation()?;

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
