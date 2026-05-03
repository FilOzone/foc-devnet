//! Foundry project setup for MockUSDFC deployment.
//!
//! This module handles the setup and preparation of the Foundry project
//! for deploying the MockUSDFC contract.

use crate::embedded_assets;
use crate::paths::foc_devnet_run_dir;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Get or create the MockUSDFC project directory from embedded assets
///
/// Extracts the embedded MockUSDFC Foundry project to a temporary directory
/// and returns the path to that directory.
pub fn get_mockusdfc_project_dir(run_id: &str) -> Result<PathBuf, Box<dyn Error>> {
    let run_dir = foc_devnet_run_dir(run_id);
    let extract_target = run_dir.join("mockusdfc-extract");

    // Always clean and re-extract to ensure we have the latest embedded version
    if extract_target.exists() {
        fs::remove_dir_all(&extract_target)?;
    }

    // Extract embedded MockUSDFC project (this creates contracts/MockUSDFC/ subdirectory)
    embedded_assets::extract_mockusdfc_project(&extract_target)?;

    // The actual project is in the contracts/MockUSDFC subdirectory
    let mockusdfc_dir = extract_target.join("contracts").join("MockUSDFC");

    if !mockusdfc_dir.exists() {
        return Err(format!(
            "MockUSDFC directory not found after extraction at: {}",
            mockusdfc_dir.display()
        )
        .into());
    }

    Ok(mockusdfc_dir)
}
