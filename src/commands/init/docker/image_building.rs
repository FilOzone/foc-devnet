//! Docker image building utilities.
//!
//! This module handles the building of Docker images from embedded Dockerfiles.

use super::image_checking::image_exists;
use crate::embedded_assets;
use crate::shell::{build_docker_image_with_args, get_current_gid, get_current_uid};
use crossterm::style::Stylize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;

/// Build a Docker image from embedded Dockerfile.
///
/// # Arguments
/// * `name` - Name for the image (used in tagging and to get the embedded Dockerfile)
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
pub fn build_docker_image_from_embedded(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let image_tag = format!("foc-{}", name);

    // Check if image already exists in Docker
    if image_exists(&image_tag) {
        println!(
            "    {} Docker image {} already exists, skipping build",
            "✓".green(),
            image_tag
        );
    } else {
        perform_docker_build_from_embedded(name, &image_tag)?;
    }

    Ok(())
}

/// Perform the actual Docker build process from embedded Dockerfile.
///
/// # Arguments
/// * `name` - Name of the embedded Dockerfile
/// * `image_tag` - Tag for the built image
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
pub fn perform_docker_build_from_embedded(
    name: &str,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dockerfile_content = embedded_assets::get_dockerfile(name)
        .ok_or_else(|| format!("Embedded Dockerfile not found for: {}", name))?;

    println!(
        "    {} Building Docker image: {} from embedded Dockerfile.{}",
        "🔨".bold(),
        image_tag,
        name
    );

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Building Docker image: {}", image_tag));

    // Get current user's UID and GID for non-root container execution
    let uid = get_current_uid()?;
    let gid = get_current_gid()?;

    // Create a temporary Dockerfile
    let temp_dockerfile_path = std::env::temp_dir().join(format!("Dockerfile.{}", name));
    fs::write(&temp_dockerfile_path, dockerfile_content)?;

    let build_args = vec![("USER_ID", uid.as_str()), ("GROUP_ID", gid.as_str())];
    let output = build_docker_image_with_args(
        &temp_dockerfile_path.to_string_lossy(),
        image_tag,
        ".", // Build context is current directory
        &build_args,
    )?;

    // Clean up temp file
    let _ = fs::remove_file(&temp_dockerfile_path);

    if !output.status.success() {
        pb.finish_with_message(format!("❌ Failed to build Docker image: {}", image_tag));
        return Err(format!("Failed to build Docker image: {}", image_tag).into());
    }

    pb.finish_with_message(format!("✓ Built image: {}", image_tag));
    Ok(())
}

/// Build the YugabyteDB Docker image with special context handling.
///
/// This function builds from the artifacts directory to include the yugabyte folder.
///
/// # Arguments
/// * `name` - Name for the image (used in tagging)
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
pub fn build_yugabyte_docker_image(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let image_tag = format!("foc-{}", name);

    // Check if image already exists in Docker
    if image_exists(&image_tag) {
        println!(
            "    {} Docker image {} already exists, skipping build",
            "✓".green(),
            image_tag.clone().blue()
        );
    } else {
        perform_yugabyte_docker_build_from_embedded(name, &image_tag)?;
    }

    Ok(())
}

/// Perform the YugabyteDB Docker build with special context handling from embedded Dockerfile.
///
/// # Arguments
/// * `name` - Name of the embedded Dockerfile
/// * `image_tag` - Tag for the built image
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
pub fn perform_yugabyte_docker_build_from_embedded(
    name: &str,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::paths::foc_localnet_artifacts;

    let dockerfile_content = embedded_assets::get_dockerfile(name)
        .ok_or_else(|| format!("Embedded Dockerfile not found for: {}", name))?;

    let artifacts_dir = foc_localnet_artifacts();

    // Check if yugabyte directory exists in artifacts
    let yugabyte_dir = artifacts_dir.join("yugabyte");
    if !yugabyte_dir.exists() {
        return Err(format!(
            "Yugabyte directory not found at {}. Please ensure artifacts are downloaded first.",
            yugabyte_dir.display()
        )
        .into());
    }

    println!(
        "    {} Building Docker image: {} from embedded Dockerfile.{}",
        "🔨".bold(),
        image_tag,
        name
    );
    println!(
        "    {} Using build context: {}",
        "📁".bold(),
        artifacts_dir.display()
    );

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Building Docker image: {}", image_tag));

    // Get current user's UID and GID for non-root container execution
    let uid = get_current_uid()?;
    let gid = get_current_gid()?;

    // Create a temporary Dockerfile
    let temp_dockerfile_path = std::env::temp_dir().join(format!("Dockerfile.{}", name));
    fs::write(&temp_dockerfile_path, dockerfile_content)?;

    // Build from artifacts directory as context to include yugabyte folder
    let build_args = vec![("USER_ID", uid.as_str()), ("GROUP_ID", gid.as_str())];
    let output = build_docker_image_with_args(
        &temp_dockerfile_path.to_string_lossy(),
        image_tag,
        &artifacts_dir.to_string_lossy(),
        &build_args,
    )?;

    // Clean up temp file
    let _ = fs::remove_file(&temp_dockerfile_path);

    if !output.status.success() {
        pb.finish_with_message(format!("❌ Failed to build Docker image: {}", image_tag));
        return Err(format!("Failed to build Docker image: {}", image_tag).into());
    }

    pb.finish_with_message(format!("✓ Built image: {}", image_tag));
    Ok(())
}
