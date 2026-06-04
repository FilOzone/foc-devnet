//! Docker image building utilities.
//!
//! This module provides functions for building Docker images, including
//! support for embedded Dockerfiles and custom build arguments.

use crate::docker::core::{docker_command, get_current_gid, get_current_uid, image_exists};
use crate::embedded_assets;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// Build a Docker image from embedded Dockerfile.
///
/// # Arguments
/// * `name` - Name for the image (used in tagging and to get the embedded Dockerfile)
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
pub fn build_image_from_embedded(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let image_tag = format!("foc-{}", name);

    if image_exists(&image_tag)? {
        info!("Docker image {} already exists, skipping build", image_tag);
    } else {
        perform_build_from_embedded(name, &image_tag)?;
    }
    Ok(())
}

/// Perform the actual Docker build process from embedded Dockerfile.
pub fn perform_build_from_embedded(
    name: &str,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dockerfile_content = embedded_assets::get_dockerfile(name)
        .ok_or_else(|| format!("Embedded Dockerfile not found for: {}", name))?;

    print_build_info(name, image_tag);
    let pb = setup_build_progress_bar(image_tag);
    let (uid, gid) = get_build_user_ids()?;
    let temp_dockerfile_path =
        create_temp_dockerfile(name, std::str::from_utf8(dockerfile_content)?)?;

    let result = execute_standard_build(&temp_dockerfile_path, image_tag, &uid, &gid);

    let _ = fs::remove_file(&temp_dockerfile_path);
    finalize_build_progress(&pb, image_tag, result)
}

/// Print build information for standard image.
fn print_build_info(_name: &str, image_tag: &str) {
    info!(
        "Building Docker image: {} from embedded Dockerfile",
        image_tag
    );
}

/// Execute a standard Docker build.
fn execute_standard_build(
    dockerfile_path: &Path,
    image_tag: &str,
    uid: &str,
    gid: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let build_args = vec![("USER_ID", uid), ("GROUP_ID", gid)];
    let output = build_image_with_args(
        &dockerfile_path.to_string_lossy(),
        image_tag,
        ".",
        &build_args,
    )?;

    if !output.status.success() {
        return Err(format!("Failed to build Docker image: {}", image_tag).into());
    }
    Ok(())
}

/// Build Docker image with custom build arguments.
pub fn build_image_with_args(
    dockerfile_path: &str,
    image_tag: &str,
    build_context: &str,
    build_args: &[(&str, &str)],
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let mut args = vec!["build", "--progress", "plain"];

    let formatted_args: Vec<String> = build_args
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect();

    for formatted_arg in &formatted_args {
        args.push("--build-arg");
        args.push(formatted_arg);
    }

    args.extend_from_slice(&["--file", dockerfile_path, "--tag", image_tag, build_context]);
    docker_command(&args)
}

/// Set up progress bar for Docker build.
fn setup_build_progress_bar(image_tag: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Building Docker image: {}", image_tag));
    pb
}

/// Get user and group IDs for Docker build.
fn get_build_user_ids() -> Result<(String, String), Box<dyn std::error::Error>> {
    let uid = get_current_uid()?;
    let gid = get_current_gid()?;
    Ok((uid, gid))
}

/// Create temporary Dockerfile for build.
fn create_temp_dockerfile(
    name: &str,
    content: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let temp_path = std::env::temp_dir().join(format!("Dockerfile.{}", name));
    fs::write(&temp_path, content)?;
    Ok(temp_path)
}

/// Finalize build progress and report results.
fn finalize_build_progress(
    pb: &ProgressBar,
    image_tag: &str,
    result: Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    match result {
        Ok(()) => {
            pb.finish_with_message(format!("✓ Built image: {}", image_tag));
            info!("Successfully built Docker image: {}", image_tag);
            Ok(())
        }
        Err(e) => {
            pb.finish_with_message(format!("❌ Failed to build Docker image: {}", image_tag));
            error!("Failed to build Docker image {}: {}", image_tag, e);
            Err(e)
        }
    }
}

/// Build and cache all required Docker images for foc-devnet.
///
/// This function builds the following images from embedded Dockerfiles:
/// - BUILDER_DOCKER_IMAGE (Foundry tools)
/// - LOTUS_DOCKER_IMAGE (Filecoin daemon)
/// - LOTUS_MINER_DOCKER_IMAGE (Filecoin miner)
/// - CURIO_DOCKER_IMAGE (Second-generation miner)
pub fn build_and_cache_docker_images() -> Result<(), Box<dyn std::error::Error>> {
    info!("Building and caching Docker images...");

    let images = ["builder", "lotus", "lotus-miner", "curio"];

    for image_name in &images {
        info!("Building image: foc-{}", image_name);
        build_image_from_embedded(image_name)?;
    }

    info!("✓ All Docker images built and cached");
    Ok(())
}

/// Build a Docker image from a Dockerfile.
pub fn build_docker_image(
    dockerfile_path: &str,
    image_tag: &str,
    context_dir: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    use crate::docker::core::docker_command;

    docker_command(&[
        "build",
        "--progress",
        "plain",
        "-f",
        dockerfile_path,
        "-t",
        image_tag,
        context_dir,
    ])
}
