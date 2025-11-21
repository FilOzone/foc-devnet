//! Docker utilities for loading images from tar files.

use crate::paths::foc_localnet_docker_images;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

/// Load a Docker image from a tar file in the artifacts directory.
///
/// This function loads a Docker image from `~/.foc-localnet/artifacts/docker/images/<name>.tar`
/// instead of relying on Docker's cached images.
///
/// # Arguments
/// * `image_name` - The name of the image (e.g., "foc-lotus", "foc-yugabyte")
/// * `display_name` - The human-readable name for display (e.g., "Lotus", "YugabyteDB")
///
/// # Returns
/// Returns `Ok(())` if the image is loaded successfully, or an error if the tar file
/// doesn't exist or loading fails.
pub fn load_image_from_tar(image_name: &str, display_name: &str) -> Result<(), Box<dyn Error>> {
    // Extract the base name from the image tag (e.g., "foc-lotus" -> "lotus")
    let name = image_name.strip_prefix("foc-").unwrap_or(image_name);

    let images_dir = foc_localnet_docker_images();
    let tar_path = images_dir.join(format!("{}.tar", name));

    if !tar_path.exists() {
        return Err(format!(
            "Docker image tar file not found at {}. Please run 'foc-localnet init' first.",
            tar_path.display()
        )
        .into());
    }

    println!(
        "    Loading {} Docker image from {}...",
        display_name,
        tar_path.display()
    );

    let output = Command::new("docker")
        .args(["load", "-i", &tar_path.to_string_lossy()])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to load {} Docker image: {}",
            display_name,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("    {} {} Docker image loaded", "✓".green(), display_name);
    Ok(())
}
