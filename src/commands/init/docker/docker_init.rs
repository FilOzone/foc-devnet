//! Docker initialization orchestration.
//!
//! This module provides the main orchestration function for building and caching Docker images.

use crossterm::style::Stylize;

use crate::commands::init::docker::image_building::{build_docker_image_from_embedded, build_yugabyte_docker_image};
use crate::commands::init::docker::volume_management::create_volume_directories_for_images;

/// Build and cache Docker images.
///
/// This function builds Docker images from embedded Dockerfiles
/// and creates volume directories for each image.
///
/// # Returns
/// Returns `Ok(())` if all images are built successfully, or an error if any step fails.
pub fn build_and_cache_docker_images() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Building Docker images...".bold());

    // Get all available Dockerfiles from embedded assets
    let dockerfile_names = ["builder", "curio", "lotus", "lotus-miner", "yugabyte"];

    println!(
        "  {} Found {} Dockerfile(s) to build:",
        "✓".green(),
        dockerfile_names.len()
    );

    for dockerfile_name in dockerfile_names {
        // Build the Docker image with special handling for yugabyte
        match dockerfile_name {
            "yugabyte" => {
                build_yugabyte_docker_image(dockerfile_name)?;
            }
            _ => {
                build_docker_image_from_embedded(dockerfile_name)?;
            }
        }
    }

    println!("  {} Docker images built", "✓".green());

    // Create and initialize volume directories AFTER images are built
    create_volume_directories_for_images()?;
    Ok(())
}