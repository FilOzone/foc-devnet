//! Docker image status utilities.
//!
//! This module provides utilities for checking Docker image existence.

use std::process::Command;

/// Check if a Docker image exists locally in the Docker daemon.
///
/// This function checks if the Docker image exists in the local Docker daemon
/// using `docker images`.
///
/// # Arguments
/// * `image_tag` - The tag of the Docker image to check (e.g., "foc-builder")
///
/// # Returns
/// Returns `true` if the image exists in the local Docker daemon, `false` otherwise.
pub fn image_exists(image_tag: &str) -> bool {
    let output = Command::new("docker")
        .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .any(|line| line.starts_with(&format!("{}:", image_tag)))
        }
        _ => false,
    }
}
