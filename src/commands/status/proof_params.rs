//! Proof parameters status display for foc-localnet.
//!
//! This module displays the proof parameters docker volume status,
//! including availability and validation.

use crate::paths::foc_localnet_proof_parameters;
use std::fs;
use tracing::{info, warn};

/// Print the proof parameters status.
///
/// Displays whether the proof parameters docker volume is available and valid.
pub fn print_proof_params_status() -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = foc_localnet_proof_parameters();

    if !params_dir.exists() {
        warn!("Status: NOT AVAILABLE");
        info!("Run 'foc-localnet init' to download proof parameters.");
        return Ok(());
    }

    // Validate proof parameters
    match validate_proof_parameters(&params_dir) {
        Ok(true) => {
            info!("FilProofParams: OK, at: {}", params_dir.display());
        }
        Ok(false) => {
            warn!(
                "FilProofParams: INVALID (missing or incomplete files), available at: {}",
                params_dir.display()
            );
            info!("Run 'foc-localnet init' to re-download proof parameters.");
        }
        Err(e) => {
            warn!("FilProofParams: UNKNOWN (validation error: {})", e);
        }
    }

    Ok(())
}

/// Validate that proof parameters directory contains expected files.
///
/// This performs a heuristic validation without requiring exact file matches:
/// - Checks for at least one large .params file (> 10MB)
/// - Checks for at least one .srs file (> 100MB)
/// - Checks for multiple .vk (verification key) files (>= 5)
/// - Verifies files follow v28- naming convention
/// - Ensures total directory size is reasonable (> 1GB)
fn validate_proof_parameters(
    params_dir: &std::path::Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !params_dir.exists() || !params_dir.is_dir() {
        return Ok(false);
    }

    let entries: Vec<_> = fs::read_dir(params_dir)?.filter_map(|e| e.ok()).collect();

    if entries.is_empty() {
        return Ok(false);
    }

    let mut has_large_params = false;
    let mut has_srs_file = false;
    let mut vk_count = 0;
    let mut total_size = 0u64;

    for entry in entries {
        let path = entry.path();
        let metadata = entry.metadata()?;

        if !metadata.is_file() {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let file_size = metadata.len();
        total_size += file_size;

        // Verify v28- naming convention
        if !file_name.starts_with("v28-") {
            continue;
        }

        // Check for .params files (should be large, > 10MB)
        if file_name.ends_with(".params") && file_size > 10_000_000 {
            has_large_params = true;
        }

        // Check for .srs files (should be very large, > 100MB)
        if file_name.ends_with(".srs") && file_size > 100_000_000 {
            has_srs_file = true;
        }

        // Count .vk (verification key) files
        if file_name.ends_with(".vk") {
            vk_count += 1;
        }
    }

    // Validation checks
    let is_valid = has_large_params && has_srs_file && vk_count >= 5 && total_size > 1_000_000_000; // At least 1GB

    Ok(is_valid)
}
