//! Docker image status utilities.
//!
//! This module provides utilities for checking Docker image existence.
//! Uses the centralized docker utilities to avoid duplication.

use crate::docker;

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
    docker::image_exists(image_tag)
}
