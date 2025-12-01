//! Docker image existence checking utilities.
//!
//! This module provides functions to check if Docker images exist locally.
//! Uses the centralized docker utilities to avoid duplication.

use crate::docker;

/// Check if a Docker image exists locally in the Docker daemon.
///
/// # Arguments
/// * `image_tag` - The tag of the Docker image to check (e.g., "foc-builder")
///
/// # Returns
/// Returns `true` if the image exists in the local Docker daemon, `false` otherwise.
pub fn image_exists(image_tag: &str) -> bool {
    docker::image_exists(image_tag)
}
