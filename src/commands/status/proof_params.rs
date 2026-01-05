//! Proof parameters status display for foc-localnet.
//!
//! This module displays the proof parameters docker volume status,
//! including availability and validation.

use crate::paths::foc_localnet_proof_parameters;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::process::Command;
use tracing::{info, warn};

/// Expected SHA256 hash of the proof parameters directory
const EXPECTED_PROOF_PARAMS_SHA256: &str =
    "1d2746e3c92f96608adfa10849004b5654104082c3bba5a2e5362e0657741527";

/// Compute the SHA256 hash of all files in the proof parameters directory
///
/// This function computes a deterministic hash based only on file contents (not paths) by:
/// 1. Finding all regular files: `find params_dir -type f -exec sha256sum {} \;`
/// 2. Extracting only the hash part (first field), removing file paths
/// 3. Sorting the hashes to ensure consistent ordering
/// 4. Computing SHA256 of the concatenated sorted hashes
fn compute_proof_params_hash(params_dir: &std::path::Path) -> Result<String, Box<dyn Error>> {
    let output = Command::new("find")
        .arg(params_dir)
        .arg("-type")
        .arg("f")
        .arg("-exec")
        .arg("sha256sum")
        .arg("{}")
        .arg(";")
        .output()?;

    if !output.status.success() {
        return Err("Failed to compute file hashes".into());
    }

    let file_hashes = String::from_utf8(output.stdout)?;
    // Extract only the hash part (first field), not the file path
    let mut hashes: Vec<&str> = file_hashes
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    hashes.sort();

    let combined = hashes.join("\n");
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Print the proof parameters status.
///
/// Displays whether the proof parameters docker volume is available and valid.
pub fn print_proof_params_status() -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = foc_localnet_proof_parameters();

    if !params_dir.exists() {
        info!("Run 'foc-localnet start' to download proof parameters. (first run downloads and caches it for future runs)");
        return Ok(());
    }

    // Validate proof parameters
    match validate_proof_parameters(&params_dir) {
        Ok(true) => {
            info!("FilProofParams: OK, at: {}", params_dir.display());
        }
        Ok(false) => {
            warn!(
                "FilProofParams: INVALID (hash mismatch or missing), available at: {}",
                params_dir.display()
            );
            info!("Run 'rm -rf ~/.foc-localnet/docker/volumes/cache/filecoin-proof-parameters; foc-localnet start' to re-download proof parameters.");
        }
        Err(e) => {
            warn!("FilProofParams: UNKNOWN (validation error: {})", e);
        }
    }

    Ok(())
}

/// Validate that proof parameters directory contains expected files and matches expected hash.
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

    // Check hash
    let computed_hash = compute_proof_params_hash(params_dir)?;
    if computed_hash != EXPECTED_PROOF_PARAMS_SHA256 {
        return Ok(false);
    }

    Ok(true)
}
