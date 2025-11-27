//! Docker image building utilities for foc-localnet initialization.
//!
//! This module handles the building, caching, and volume setup for Docker images
//! required by foc-localnet.

use crossterm::style::Stylize;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
/// This function builds Docker images from Dockerfiles in the docker/ directory
/// and creates volume directories for each image.
///
/// # Returns
/// Returns `Ok(())` if all images are built successfully, or an error if any step fails.
pub fn build_and_cache_docker_images() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Building Docker images...".bold());

    // Find all Dockerfile files in the docker directory
    let docker_dir = Path::new("docker");
    if !docker_dir.exists() {
        println!(
            "  {} docker/ directory not found, skipping Docker image building",
            "⚠".yellow()
        );
        return Ok(());
    }

    let dockerfiles = find_dockerfiles(docker_dir)?;

    if dockerfiles.is_empty() {
        println!(
            "{}",
            "  No Dockerfile files found in docker/ directory".yellow()
        );
        return Ok(());
    }

    println!(
        "  {} Found {} Dockerfile(s) to build:",
        "✓".green(),
        dockerfiles.len()
    );

    for dockerfile in dockerfiles {
        let dockerfile_suffix = extract_name(&dockerfile)?;

        // Build the Docker image with special handling for yugabyte

        match dockerfile_suffix.as_str() {
            "yugabyte" => {
                build_yugabyte_docker_image(&dockerfile, &dockerfile_suffix)?;
            }
            _ => {
                build_docker_image(&dockerfile, &dockerfile_suffix)?;
            }
        }
    }

    println!("  {} Docker images built", "✓".green());

    // Create and initialize volume directories AFTER images are built
    create_volume_directories_for_images()?;
    Ok(())
}

/// Find all files named Dockerfile or Dockerfile.<name> in the given directory.
///
/// # Arguments
/// * `dir` - Directory to search for Dockerfiles
///
/// # Returns
/// Returns a vector of paths to Dockerfile files.
fn find_dockerfiles(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut dockerfiles = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename == "Dockerfile" || filename.starts_with("Dockerfile.") {
                    dockerfiles.push(path);
                }
            }
        }
    }

    Ok(dockerfiles)
}

/// Extract the name from a Dockerfile.<name> path.
/// Special case: plain "Dockerfile" becomes "builder".
///
/// # Arguments
/// * `dockerfile_path` - Path to the Dockerfile
///
/// # Returns
/// Returns the extracted name, or an error if the path is invalid.
fn extract_name(dockerfile_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let filename = dockerfile_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid dockerfile path")?;

    if filename == "Dockerfile" {
        Ok("builder".to_string())
    } else if let Some(name) = filename.strip_prefix("Dockerfile.") {
        Ok(name.to_string())
    } else {
        Err(format!("Invalid dockerfile name: {}", filename).into())
    }
}

/// Build a Docker image from the given Dockerfile.
///
/// # Arguments
/// * `dockerfile_path` - Path to the Dockerfile
/// * `name` - Name for the image (used in tagging)
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
fn build_docker_image(
    dockerfile_path: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let image_tag = format!("foc-{}", name);

    // Check if image already exists in Docker
    if image_exists(&image_tag) {
        println!(
            "    {} Docker image {} already exists, skipping build",
            "✓".green(),
            image_tag
        );
    } else {
        perform_docker_build(dockerfile_path, &image_tag)?;
    }

    Ok(())
}

/// Perform the actual Docker build process.
///
/// # Arguments
/// * `dockerfile_path` - Path to the Dockerfile
/// * `image_tag` - Tag for the built image
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
fn perform_docker_build(
    dockerfile_path: &Path,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dockerfile_dir = dockerfile_path.parent().unwrap_or(Path::new("."));

    println!(
        "    {} Building Docker image: {} from {}",
        "🔨".bold(),
        image_tag,
        dockerfile_path.display()
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
            &dockerfile_path.to_string_lossy(),
            "--tag",
            image_tag,
            &dockerfile_dir.to_string_lossy(),
        ])
        .status()?;

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
/// * `dockerfile_path` - Path to the Dockerfile
/// * `name` - Name for the image (used in tagging)
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
fn build_yugabyte_docker_image(
    dockerfile_path: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let image_tag = format!("foc-{}", name);

    // Check if image already exists in Docker
    if image_exists(&image_tag) {
        println!(
            "    {} Docker image {} already exists, skipping build",
            "✓".green(),
            image_tag.clone().blue()
        );
    } else {
        perform_yugabyte_docker_build(dockerfile_path, &image_tag)?;
    }

    Ok(())
}

/// Perform the YugabyteDB Docker build with special context handling.
///
/// # Arguments
/// * `dockerfile_path` - Path to the Dockerfile
/// * `image_tag` - Tag for the built image
///
/// # Returns
/// Returns `Ok(())` if build succeeds, or an error if build fails.
fn perform_yugabyte_docker_build(
    dockerfile_path: &Path,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
        "    {} Building Docker image: {} from {}",
        "🔨".bold(),
        image_tag,
        dockerfile_path.display()
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
            &dockerfile_path.to_string_lossy(),
            "--tag",
            image_tag,
            &artifacts_dir.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        pb.finish_with_message(format!("❌ Failed to build Docker image: {}", image_tag));
        return Err(format!("Failed to build Docker image: {}", image_tag).into());
    }

    pb.finish_with_message(format!("✓ Built image: {}", image_tag));
    Ok(())
}

/// Create volume directories for all Docker images based on their volume map files.
///
/// This function scans the docker/ directory for .volumes_map.toml files and
/// creates the corresponding volume directories.
fn create_volume_directories_for_images() -> Result<(), Box<dyn std::error::Error>> {
    let docker_dir = Path::new("docker");
    if !docker_dir.exists() {
        return Ok(());
    }

    let volumes_base_dir = foc_localnet_docker_volumes();

    // Find all .volumes_map files in the docker directory
    for entry in fs::read_dir(docker_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.ends_with(".volumes_map.toml") {
                    // Extract image name from filename (e.g., "foc-builder.volumes_map.toml" -> "foc-builder")
                    if let Some(image_name) = filename.strip_suffix(".volumes_map.toml") {
                        create_volumes_for_image(image_name, &path, &volumes_base_dir)?;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Create volume directories for a specific image based on its volume map file.
///
/// # Arguments
/// * `image_name` - Name of the Docker image
/// * `volumes_map_path` - Path to the volumes map TOML file
/// * `volumes_base_dir` - Base directory for all volumes
fn create_volumes_for_image(
    image_name: &str,
    volumes_map_path: &Path,
    volumes_base_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(volumes_map_path)?;

    #[derive(serde::Deserialize)]
    struct VolumesMap {
        volumes: HashMap<String, String>,
    }

    let volume_config: VolumesMap = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", volumes_map_path.display(), e))?;

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
    let _ = Command::new("docker")
        .args(["rm", container_name])
        .status();
}
