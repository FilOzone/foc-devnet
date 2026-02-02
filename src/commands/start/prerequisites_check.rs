//! Binary and Docker image availability check for cluster startup.
//!
//! This module provides a unified check for all required binaries and Docker images
//! before starting the cluster. It provides clear error messages directing users to
//! build missing components.

use super::step::{SetupContext, Step};
use crate::constants::{REQUIRED_BINARIES, REQUIRED_DOCKER_IMAGES};
use crate::docker::core::image_exists;
use crate::paths::foc_devnet_bin;
use std::error::Error;
use tracing::info;

/// Check that all required binaries exist in the bin directory.
///
/// This check runs early in cluster startup before any containers are started,
/// ensuring that we fail fast with a helpful message if binaries are missing.
///
/// # Errors
///
/// Returns an error if any required binary is missing, listing all missing binaries
/// and providing instructions on how to build them.
fn check_all_binaries() -> Result<(), Box<dyn Error>> {
    let bin_dir = foc_devnet_bin();

    let mut missing_binaries = Vec::new();
    let mut found_binaries = Vec::new();

    for binary_name in REQUIRED_BINARIES {
        let binary_path = bin_dir.join(binary_name);
        if binary_path.exists() {
            found_binaries.push(*binary_name);
        } else {
            missing_binaries.push(*binary_name);
        }
    }

    // Log what we found
    for binary in &found_binaries {
        info!("✓ Binary '{}' found", binary);
    }

    // If any binaries are missing, return an error with instructions
    if !missing_binaries.is_empty() {
        let missing_list = missing_binaries.join("', '");
        return Err(format!(
            "Missing required binaries: '{}'\n\nPlease build them with 'foc-devnet build <name>' \
             (e.g., 'foc-devnet build lotus' or 'foc-devnet build curio')",
            missing_list
        )
        .into());
    }

    info!("✓ All required binaries are available");
    Ok(())
}

/// Check that all required Docker images exist.
///
/// This check runs early in cluster startup before any containers are started,
/// ensuring that we fail fast with a helpful message if Docker images are missing.
///
/// # Errors
///
/// Returns an error if any required Docker image is missing, listing all missing images
/// and providing instructions on how to build them.
fn check_all_docker_images() -> Result<(), Box<dyn Error>> {
    let mut missing_images = Vec::new();
    let mut found_images = Vec::new();

    for image_name in REQUIRED_DOCKER_IMAGES {
        let exists = image_exists(image_name)
            .map_err(|e| format!("Failed to check Docker image '{}': {}", image_name, e))?;

        if exists {
            found_images.push(*image_name);
        } else {
            missing_images.push(*image_name);
        }
    }

    // Log what we found
    for image in &found_images {
        info!("✓ Docker image '{}' found", image);
    }

    // If any images are missing, return an error with instructions
    if !missing_images.is_empty() {
        let missing_list = missing_images.join("', '");
        return Err(format!(
            "Missing required Docker images: '{}'\n\nPlease run 'foc-devnet init' to build all Docker images.",
            missing_list
        )
        .into());
    }

    info!("✓ All required Docker images are available");
    Ok(())
}

/// Prerequisites check step for cluster startup.
///
/// This step verifies that all required binaries and Docker images are available
/// before starting any containers. It runs as the very first step in the startup sequence.
pub struct PrerequisitesCheckStep;

impl PrerequisitesCheckStep {
    /// Create a new PrerequisitesCheckStep
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrerequisitesCheckStep {
    fn default() -> Self {
        Self::new()
    }
}

impl Step for PrerequisitesCheckStep {
    fn name(&self) -> &str {
        "Prerequisites Check (Binaries & Docker Images)"
    }

    fn pre_execute(&self, _context: &SetupContext) -> Result<(), Box<dyn Error>> {
        // No pre-checks needed
        Ok(())
    }

    fn execute(&self, _context: &SetupContext) -> Result<(), Box<dyn Error>> {
        check_all_binaries()?;
        check_all_docker_images()?;
        Ok(())
    }

    fn post_execute(&self, _context: &SetupContext) -> Result<(), Box<dyn Error>> {
        // No post-checks needed
        Ok(())
    }
}
