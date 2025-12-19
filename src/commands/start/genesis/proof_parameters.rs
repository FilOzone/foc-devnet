//! Proof parameters management for genesis preparation.
//!
//! This module handles downloading and caching Filecoin proof parameters
//! required for lotus operations.

use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_proof_parameters,
    CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::info;

/// Ensure Filecoin proof parameters are downloaded.
///
/// Parameters are downloaded once and cached in ~/.foc-localnet/artifacts/filecoin-proof-parameters/
/// This directory is mounted into lotus containers at /var/tmp/filecoin-proof-parameters/
pub fn ensure_proof_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = foc_localnet_proof_parameters();

    // Check if parameters already exist and are valid
    if params_dir.exists() && validate_proof_parameters(&params_dir)? {
        info!("✓ Proof parameters already exist locally",);
        return Ok(());
    }

    info!("⬇ Downloading proof parameters (this may take a while)...");

    // Create the directory
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
        return Err(format!(
            "Failed to download proof parameters: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

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

        // Check for .srs file (should be substantial, > 100MB)
        if file_name.ends_with(".srs") && file_size > 100_000_000 {
            has_srs_file = true;
        }

        // Count .vk (verification key) files
        if file_name.ends_with(".vk") {
            vk_count += 1;
        }
    }

    // Validation criteria (very crude):
    // - At least one large .params file
    // - At least one .srs file
    // - At least 5 .vk files
    // - Total directory size > 1GB
    let is_valid = has_large_params && has_srs_file && vk_count >= 5 && total_size > 1_000_000_000;

    Ok(is_valid)
}
