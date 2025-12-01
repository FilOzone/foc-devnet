//! Docker volume management utilities.
//!
//! This module handles the creation and initialization of Docker volume directories.

use super::container_utils::{cleanup_temp_container, create_temp_container, perform_volume_copy};
use crate::embedded_assets;
use crate::paths::foc_localnet_docker_volumes;
use crossterm::style::Stylize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Create volume directories for all Docker images based on their embedded volume map files.
///
/// This function iterates over all known volume map files in embedded assets
/// and creates the corresponding volume directories.
pub fn create_volume_directories_for_images() -> Result<(), Box<dyn std::error::Error>> {
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
pub fn create_volumes_for_image_from_embedded(
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

    let volume_config: VolumesMap = toml::from_str(content).map_err(|e| {
        format!(
            "Failed to parse embedded volumes map for {}: {}",
            image_name, e
        )
    })?;

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
pub fn set_volume_ownership(volume_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
pub fn copy_initial_volume_contents(
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