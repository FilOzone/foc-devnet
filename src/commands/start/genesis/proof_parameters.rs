//! Proof parameters management for genesis preparation.
//!
//! This module handles downloading and caching Filecoin proof parameters
//! required for lotus operations.

use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_proof_parameters,
    CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
};
use crate::utils::retry::{retry_with_fixed_delay, DEFAULT_MAX_RETRIES, DEFAULT_RETRY_DELAY_SECS};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::fs;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
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
fn compute_proof_params_hash(
    params_dir: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Command;

    info!("Computing hash for directory: {}", params_dir.display());

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
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to compute file hashes: {}", stderr).into());
    }

    let file_hashes = String::from_utf8(output.stdout)?;
    let file_count = file_hashes.lines().count();
    info!("Found {} files in proof parameters directory", file_count);

    // Extract only the hash part (first field), not the file path
    let mut hashes: Vec<&str> = file_hashes
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    hashes.sort();

    // Join with newlines and add trailing newline to match bash behavior:
    // `echo hash1\nhash2\nhash3 | sha256sum` includes trailing newline
    let combined = format!("{}\n", hashes.join("\n"));
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let hash = hasher.finalize();
    let computed = format!("{:x}", hash);
    info!("Computed proof parameters hash: {}", computed);
    Ok(computed)
}

/// Ensure Filecoin proof parameters are downloaded.
///
/// Parameters are downloaded once and cached in ~/.foc-localnet/artifacts/filecoin-proof-parameters/
/// This directory is mounted into lotus containers at /var/tmp/filecoin-proof-parameters/
pub fn ensure_proof_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = foc_localnet_proof_parameters();

    // Check if parameters already exist and are valid
    if params_dir.exists() {
        info!(
            "Checking existing proof parameters at: {}",
            params_dir.display()
        );
        match validate_proof_parameters(&params_dir) {
            Ok(true) => {
                info!("✓ Proof parameters already exist and are valid");
                return Ok(());
            }
            Ok(false) => {
                info!("Proof parameters exist but validation failed, will re-download");
            }
            Err(e) => {
                warn!("Error validating proof parameters: {}, will re-download", e);
            }
        }
    } else {
        info!(
            "Proof parameters directory does not exist: {}",
            params_dir.display()
        );
    }

    info!("⬇ Downloading proof parameters (this may take a while)...");

    // Retry the download operation in case of network issues
    retry_with_fixed_delay(
        || {
            // Ensure directory exists for each attempt (in case cleanup removed it)
            fs::create_dir_all(&params_dir)?;

            // Run lotus fetch-params in builder container
            let bin_dir = foc_localnet_bin();
            let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

            // Create a progress bar
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
            );

            let bytes_downloaded = Arc::new(Mutex::new(0u64));
            let start_time = Instant::now();
            let bytes_clone = Arc::clone(&bytes_downloaded);
            let params_dir_clone = params_dir.clone();

            // Spawn a thread to update progress by monitoring directory size
            let pb_clone = pb.clone();
            let update_handle = thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_millis(500));

                    // Calculate directory size
                    if let Ok(size) = get_dir_size(&params_dir_clone) {
                        let mut total = bytes_clone.lock().unwrap();
                        *total = size;

                        let elapsed = start_time.elapsed().as_secs_f64();
                        if elapsed > 0.0 {
                            let speed_mbps = (size as f64 / 1_048_576.0) / elapsed;
                            let total_mb = size as f64 / 1_048_576.0;
                            pb_clone.set_message(format!(
                                "Downloaded {:.1} MB ({:.2} MB/s)",
                                total_mb, speed_mbps
                            ));
                        }
                    }

                    if !pb_clone.is_finished() {
                        pb_clone.tick();
                    } else {
                        break;
                    }
                }
            });

            let child = Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "-e",
                    &format!(
                        "FIL_PROOFS_PARAMETER_CACHE={}",
                        CONTAINER_FILECOIN_PROOF_PARAMS_PATH
                    ),
                    "-v",
                    &format!("{}:/output", bin_dir.display()),
                    "-v",
                    &format!(
                        "{}:/home/foc-user/.cargo",
                        builder_volumes_dir.join("cargo").display()
                    ),
                    "-v",
                    &format!(
                        "{}:{}",
                        params_dir.display(),
                        CONTAINER_FILECOIN_PROOF_PARAMS_PATH
                    ),
                    "foc-builder",
                    "/bin/bash",
                    "-c",
                    &format!(
                        "/output/lotus fetch-params {}",
                        super::constants::PROOF_PARAMS_SECTOR_SIZE
                    ),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;

            let output = child.wait_with_output()?;

            pb.finish_and_clear();
            drop(update_handle);

            if !output.status.success() {
                // Clean up partial download on failure
                if params_dir.exists() {
                    if let Err(cleanup_err) = fs::remove_dir_all(&params_dir) {
                        warn!(
                            "Failed to clean up partial proof parameters download: {}",
                            cleanup_err
                        );
                    }
                }
                return Err(format!(
                    "Failed to download proof parameters: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }

            Ok(())
        },
        DEFAULT_MAX_RETRIES,
        DEFAULT_RETRY_DELAY_SECS,
        "Proof parameters download",
    )?;

    info!("✓ Proof parameters downloaded successfully");
    Ok(())
}

/// Calculate total size of a directory recursively
fn get_dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut total_size = 0u64;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                total_size += get_dir_size(&entry.path())?;
            } else {
                total_size += metadata.len();
            }
        }
    }

    Ok(total_size)
}

/// Validate that proof parameters directory contains expected files and matches expected hash.
fn validate_proof_parameters(
    params_dir: &std::path::Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !params_dir.exists() || !params_dir.is_dir() {
        info!(
            "Proof parameters directory does not exist or is not a directory: {}",
            params_dir.display()
        );
        return Ok(false);
    }

    // Check hash
    let computed_hash = compute_proof_params_hash(params_dir)?;
    info!(
        "Proof parameters hash check: expected={}, computed={}",
        EXPECTED_PROOF_PARAMS_SHA256, computed_hash
    );
    if computed_hash != EXPECTED_PROOF_PARAMS_SHA256 {
        warn!(
            "Proof parameters hash mismatch: expected {}, got {}",
            EXPECTED_PROOF_PARAMS_SHA256, computed_hash
        );
        return Ok(false);
    }

    Ok(true)
}
