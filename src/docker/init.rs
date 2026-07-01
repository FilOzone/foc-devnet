//! Docker initialization utilities.
//!
//! This module provides functions for initializing Docker volumes,
//! containers, and other setup tasks required for foc-devnet.

use crate::docker::core::{
    chown_command, container_exists, copy_from_container, create_container, get_current_gid,
    get_current_uid, image_exists,
};
use crate::embedded_assets;
use crate::paths::foc_devnet_docker_volumes;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct VolumesMap {
    volumes: HashMap<String, String>,
}

/// Create volume directories for all Docker images.
pub fn create_volume_directories_for_images() -> Result<(), Box<dyn std::error::Error>> {
    let volumes_base_dir = foc_devnet_docker_volumes();
    let volume_map_names = ["builder", "curio", "lotus-miner", "lotus"];

    for image_name in volume_map_names {
        create_volumes_for_image_from_embedded(image_name, &volumes_base_dir)?;
    }
    Ok(())
}

/// Create volume directories for a specific image.
pub fn create_volumes_for_image_from_embedded(
    image_name: &str,
    volumes_base_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let volume_config = parse_volume_config(image_name)?;
    let docker_image_tag = format!("foc-{}", image_name);

    for (host_subdir, container_path) in volume_config.volumes.iter() {
        create_single_volume(
            &docker_image_tag,
            host_subdir,
            container_path,
            volumes_base_dir.join(image_name),
        )?;
    }
    Ok(())
}

/// Parse the embedded volume configuration for an image.
fn parse_volume_config(image_name: &str) -> Result<VolumesMap, Box<dyn std::error::Error>> {
    let content_bytes = embedded_assets::get_volumes_map(image_name)
        .ok_or_else(|| format!("Embedded volumes map not found for: {}", image_name))?;

    let content = std::str::from_utf8(content_bytes)
        .map_err(|e| format!("Invalid UTF-8 in volumes map for {}: {}", image_name, e))?;

    let volume_config: VolumesMap = toml::from_str(content)
        .map_err(|e| format!("Failed to parse volumes map for {}: {}", image_name, e))?;

    Ok(volume_config)
}

/// Create a single volume directory and initialize it if needed.
fn create_single_volume(
    docker_image_tag: &str,
    host_subdir: &str,
    container_path: &str,
    image_volume_base: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let volume_dir = image_volume_base.join(host_subdir);

    let is_new_volume = !volume_dir.exists()
        || volume_dir
            .read_dir()
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true);

    fs::create_dir_all(&volume_dir)?;
    println!("✓ Created volume directory: {}", volume_dir.display());

    set_volume_ownership(&volume_dir)?;

    if is_new_volume {
        copy_initial_volume_contents(docker_image_tag, container_path, &volume_dir)?;
    }
    Ok(())
}

/// Set the ownership of a volume directory to match the current user.
pub fn set_volume_ownership(volume_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let uid = get_current_uid()?;
    let gid = get_current_gid()?;

    let chown_arg = format!("{}:{}", uid, gid);
    let output = chown_command(&["-R", &chown_arg, &volume_dir.to_string_lossy()])?;

    if !output.status.success() {
        return Err(format!("Failed to set ownership on {}", volume_dir.display()).into());
    }
    Ok(())
}

/// Copy initial contents from a Docker image to the host volume directory.
pub fn copy_initial_volume_contents(
    image_tag: &str,
    container_path: &str,
    host_volume_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if !image_exists(image_tag)? {
        println!(
            "ℹ Image {} not yet built, skipping volume initialization",
            image_tag
        );
        return Ok(());
    }

    println!(
        "📋 Copying initial contents from {}:{} to {}",
        image_tag,
        container_path,
        host_volume_dir.display()
    );

    let container_name = create_temp_container(image_tag)?;
    let result = perform_volume_copy_and_cleanup(&container_name, container_path, host_volume_dir);

    match result {
        Ok(()) => {
            println!("✓ Initialized volume with contents from image");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Perform volume copy and handle cleanup with proper error reporting.
fn perform_volume_copy_and_cleanup(
    container_name: &str,
    container_path: &str,
    host_volume_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let copy_result = perform_volume_copy(container_name, container_path, host_volume_dir);
    cleanup_temp_container(container_name);

    match copy_result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Could not find") || stderr.contains("No such") {
                println!(
                    "ℹ No contents found at {} in image, volume remains empty",
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
pub fn create_temp_container(image_tag: &str) -> Result<String, Box<dyn std::error::Error>> {
    let container_name = format!("temp-volume-init-{}", std::process::id());
    let output = create_container(&container_name, image_tag, "/bin/true")?;

    if !output.status.success() {
        return Err("Failed to create temporary container for volume initialization".into());
    }
    Ok(container_name)
}

/// Perform the actual copy operation from container to host.
pub fn perform_volume_copy(
    container_name: &str,
    container_path: &str,
    host_volume_dir: &Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    copy_from_container(
        container_name,
        &format!("{}/.", container_path),
        &host_volume_dir.to_string_lossy(),
    )
}

/// Clean up the temporary container used for volume copying.
pub fn cleanup_temp_container(container_name: &str) {
    if container_exists(container_name).unwrap_or(false) {
        let _ = crate::docker::core::docker_command(&["rm", container_name]);
    }
}
