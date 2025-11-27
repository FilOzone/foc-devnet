//! Docker image building utilities for foc-localnet initialization.
//!
//! This module handles the building, caching, and volume setup for Docker images
//! required by foc-localnet.

use crossterm::style::Stylize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::embedded_assets;
use crate::paths::{foc_localnet_artifacts, foc_localnet_docker_volumes};

/// Check if a Docker image exists locally in the Docker daemon.
///
/// # Arguments
/// * `image_tag` - The tag of the Docker image to check (e.g., "foc-builder")
///
/// # Returns
/// Returns `true` if the image exists in the local Docker daemon, `false` otherwise.
fn image_exists(image_tag: &str) -> bool {
    Command::new("docker")
        .args(["image", "inspect", image_tag])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

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


/// Build a Docker image from embedded Dockerfile.
///
/// # Arguments
/// * `name` - Name for the image (used in tagging and to get the embedded Dockerfile)
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
fn build_docker_image_from_embedded(name: &str) -> Result<(), Box<dyn std::error::Error>> {
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
fn perform_docker_build_from_embedded(
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
    let uid_output = Command::new("id").arg("-u").output()?;
    let gid_output = Command::new("id").arg("-g").output()?;
    let uid = String::from_utf8_lossy(&uid_output.stdout)
        .trim()
        .to_string();
    let gid = String::from_utf8_lossy(&gid_output.stdout)
        .trim()
        .to_string();

    // Create a temporary Dockerfile
    let temp_dockerfile_path = std::env::temp_dir().join(format!("Dockerfile.{}", name));
    fs::write(&temp_dockerfile_path, dockerfile_content)?;
    
    let status = Command::new("docker")
        .args([
            "build",
            "--progress",
            "plain",
            "--build-arg",
            &format!("USER_ID={}", uid),
            "--build-arg",
            &format!("GROUP_ID={}", gid),
            "--file",
            &temp_dockerfile_path.to_string_lossy(),
            "--tag",
            image_tag,
            ".", // Build context is current directory
        ])
        .status()?;

    // Clean up temp file
    let _ = fs::remove_file(&temp_dockerfile_path);

    if !status.success() {
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
fn build_yugabyte_docker_image(name: &str) -> Result<(), Box<dyn std::error::Error>> {
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
fn perform_yugabyte_docker_build_from_embedded(
    name: &str,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let uid_output = Command::new("id").arg("-u").output()?;
    let gid_output = Command::new("id").arg("-g").output()?;
    let uid = String::from_utf8_lossy(&uid_output.stdout)
        .trim()
        .to_string();
    let gid = String::from_utf8_lossy(&gid_output.stdout)
        .trim()
        .to_string();

    // Create a temporary Dockerfile
    let temp_dockerfile_path = std::env::temp_dir().join(format!("Dockerfile.{}", name));
    fs::write(&temp_dockerfile_path, dockerfile_content)?;

    // Build from artifacts directory as context to include yugabyte folder
    let status = Command::new("docker")
        .args([
            "build",
            "--progress",
            "plain",
            "--build-arg",
            &format!("USER_ID={}", uid),
            "--build-arg",
            &format!("GROUP_ID={}", gid),
            "--file",
            &temp_dockerfile_path.to_string_lossy(),
            "--tag",
            image_tag,
            &artifacts_dir.to_string_lossy(),
        ])
        .status()?;

    // Clean up temp file
    let _ = fs::remove_file(&temp_dockerfile_path);

    if !status.success() {
        pb.finish_with_message(format!("❌ Failed to build Docker image: {}", image_tag));
        return Err(format!("Failed to build Docker image: {}", image_tag).into());
    }

    pb.finish_with_message(format!("✓ Built image: {}", image_tag));
    Ok(())
}

/// Create volume directories for all Docker images based on their embedded volume map files.
///
/// This function iterates over all known volume map files in embedded assets
/// and creates the corresponding volume directories.
fn create_volume_directories_for_images() -> Result<(), Box<dyn std::error::Error>> {
    let volumes_base_dir = foc_localnet_docker_volumes();

    // Get all available volume maps from embedded assets
    let volume_map_names = ["builder", "curio", "lotus-miner", "lotus", "yugabyte"];

    for image_name in volume_map_names {
        create_volumes_for_image_from_embedded(image_name, &volumes_base_dir)?;
    }

    Ok(())
}

/// Create volume directories for a specific image based on its embedded volume map file.
///
/// # Arguments
/// * `image_name` - Name of the Docker image
/// * `volumes_base_dir` - Base directory for all volumes
fn create_volumes_for_image_from_embedded(
    image_name: &str,
    volumes_base_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let content_bytes = embedded_assets::get_volumes_map(image_name)
        .ok_or_else(|| format!("Embedded volumes map not found for: {}", image_name))?;
    
    let content = std::str::from_utf8(content_bytes)
        .map_err(|e| format!("Invalid UTF-8 in volumes map for {}: {}", image_name, e))?;

    #[derive(serde::Deserialize)]
    struct VolumesMap {
        volumes: HashMap<String, String>,
    }

    let volume_config: VolumesMap = toml::from_str(content)
        .map_err(|e| format!("Failed to parse embedded volumes map for {}: {}", image_name, e))?;

    let docker_image_tag = format!("foc-{}", image_name);

    for (host_subdir, container_path) in volume_config.volumes.iter() {
        let volume_dir = volumes_base_dir.join(image_name).join(host_subdir);

        // Check if the directory is empty (new initialization)
        let is_new_volume = !volume_dir.exists()
            || volume_dir
                .read_dir()
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(true);

        fs::create_dir_all(&volume_dir)?;
        println!(
            "  {} Created volume directory: {}",
            "✓".green(),
            volume_dir.display()
        );

        // Set correct ownership on the volume directory to match the host user
        // This ensures the Docker container user can write to these directories
        set_volume_ownership(&volume_dir)?;

        // If the volume is new/empty, copy initial contents from the Docker image
        if is_new_volume {
            copy_initial_volume_contents(&docker_image_tag, container_path, &volume_dir)?;
        }
    }

    Ok(())
}

/// Set the ownership of a volume directory to match the current user.
///
/// This ensures that mounted volumes have the correct permissions for the
/// Docker container user, which is created with matching UID/GID.
///
/// # Arguments
/// * `volume_dir` - Path to the volume directory
fn set_volume_ownership(volume_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Get current user's UID and GID
    let uid_output = Command::new("id").arg("-u").output()?;
    let gid_output = Command::new("id").arg("-g").output()?;
    let uid = String::from_utf8_lossy(&uid_output.stdout)
        .trim()
        .to_string();
    let gid = String::from_utf8_lossy(&gid_output.stdout)
        .trim()
        .to_string();

    // Use chown to set ownership (requires directory to be owned by current user or have appropriate permissions)
    let chown_arg = format!("{}:{}", uid, gid);
    let status = Command::new("chown")
        .args(["-R", &chown_arg, &volume_dir.to_string_lossy()])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to set ownership on {}", volume_dir.display()).into());
    }

    Ok(())
}

/// Copy initial contents from a Docker image path to the host volume directory.
///
/// This function creates a temporary container to copy files from the image
/// to the host volume directory.
///
/// # Arguments
/// * `image_tag` - Docker image tag
/// * `container_path` - Path inside the container to copy from
/// * `host_volume_dir` - Host directory to copy to
fn copy_initial_volume_contents(
    image_tag: &str,
    container_path: &str,
    host_volume_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the image exists
    let image_exists_in_docker = Command::new("docker")
        .args(["image", "inspect", image_tag])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !image_exists_in_docker {
        println!(
            "    {} Image {} not yet built, skipping volume initialization",
            "ℹ".cyan(),
            image_tag
        );
        return Ok(());
    }

    println!(
        "    {} Copying initial contents from {}:{} to {}",
        "📋".bold(),
        image_tag,
        container_path,
        host_volume_dir.display()
    );

    let container_name = create_temp_container(image_tag)?;
    let copy_result = perform_volume_copy(&container_name, container_path, host_volume_dir);
    cleanup_temp_container(&container_name);

    match copy_result {
        Ok(output) if output.status.success() => {
            println!(
                "    {} Initialized volume with contents from image",
                "✓".green()
            );
            Ok(())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Could not find") || stderr.contains("No such") {
                println!(
                    "    {} No contents found at {} in image, volume remains empty",
                    "ℹ".cyan(),
                    container_path
                );
                Ok(())
            } else {
                Err(format!("Failed to copy volume contents: {}", stderr).into())
            }
        }
        Err(e) => Err(format!("Failed to copy volume contents: {}", e).into()),
    }
}

/// Create a temporary container for volume copying.
///
/// # Arguments
/// * `image_tag` - Docker image tag to create container from
///
/// # Returns
/// Returns the container name if successful.
fn create_temp_container(image_tag: &str) -> Result<String, Box<dyn std::error::Error>> {
    let container_name = format!("temp-volume-init-{}", std::process::id());

    // Create a container without starting it
    let create_output = Command::new("docker")
        .args(["create", "--name", &container_name, image_tag, "/bin/true"])
        .output()?;

    if !create_output.status.success() {
        return Err(
            format!("Failed to create temporary container for volume initialization").into(),
        );
    }

    Ok(container_name)
}

/// Perform the actual copy operation from container to host.
///
/// # Arguments
/// * `container_name` - Name of the temporary container
/// * `container_path` - Path inside the container to copy from
/// * `host_volume_dir` - Host directory to copy to
///
/// # Returns
/// Returns the output of the copy command.
fn perform_volume_copy(
    container_name: &str,
    container_path: &str,
    host_volume_dir: &Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    // Copy files from container to host - need to copy contents, not the directory itself
    // Use format: container:path/. to copy contents of path into destination
    let copy_output = Command::new("docker")
        .args([
            "cp",
            &format!("{}:{}/.", container_name, container_path),
            &host_volume_dir.to_string_lossy(),
        ])
        .output()?;

    Ok(copy_output)
}

/// Clean up the temporary container used for volume copying.
///
/// # Arguments
/// * `container_name` - Name of the container to remove
fn cleanup_temp_container(container_name: &str) {
    let _ = Command::new("docker").args(["rm", container_name]).status();
}
