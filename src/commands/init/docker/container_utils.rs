//! Docker container utility functions.
//!
//! This module provides utilities for creating and managing temporary Docker containers.

use crate::shell::{docker_copy_from_container, docker_create_container, docker_remove_container};

/// Create a temporary container for volume copying.
///
/// # Arguments
/// * `image_tag` - Docker image tag to create container from
///
/// # Returns
/// Returns the container name if successful.
pub fn create_temp_container(image_tag: &str) -> Result<String, Box<dyn std::error::Error>> {
    let container_name = format!("temp-volume-init-{}", std::process::id());

    // Create a container without starting it
    let create_output = docker_create_container(&container_name, image_tag, "/bin/true")?;

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
pub fn perform_volume_copy(
    container_name: &str,
    container_path: &str,
    host_volume_dir: &std::path::Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    // Copy files from container to host - need to copy contents, not the directory itself
    // Use format: container:path/. to copy contents of path into destination
    let copy_output = docker_copy_from_container(container_name, &format!("{}/.", container_path), &host_volume_dir.to_string_lossy())?;

    Ok(copy_output)
}

/// Clean up the temporary container used for volume copying.
///
/// # Arguments
/// * `container_name` - Name of the container to remove
pub fn cleanup_temp_container(container_name: &str) {
    let _ = docker_remove_container(container_name);
}
