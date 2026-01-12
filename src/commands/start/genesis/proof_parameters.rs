//! Proof parameters management for genesis preparation.
//!
//! This module handles downloading and caching Filecoin proof parameters
//! required for lotus operations.

use crate::paths::{
    foc_devnet_bin, foc_devnet_docker_volumes, foc_devnet_proof_parameters,
    CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
};
use crate::utils::retry::{retry_with_fixed_delay, DEFAULT_MAX_RETRIES, DEFAULT_RETRY_DELAY_SECS};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// S3 URL for pre-packaged Filecoin proof parameters (2KiB sectors)
const PROOF_PARAMS_S3_URL: &str =
    "https://fil-proof-params-2k-cache.s3.us-east-2.amazonaws.com/filecoin-proof-params-2k.tar";

/// Ensure Filecoin proof parameters are downloaded.
///
/// Parameters are downloaded once and cached in ~/.foc-devnet/artifacts/filecoin-proof-parameters/
/// This directory is mounted into lotus containers at /var/tmp/filecoin-proof-parameters/
pub fn ensure_proof_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = foc_devnet_proof_parameters();

    // Check if parameters already exist
    if params_dir.exists() && params_dir.read_dir()?.next().is_some() {
        info!(
            "✓ Proof parameters already exist at: {}",
            params_dir.display()
        );
        return Ok(());
    }

    info!(
        "Proof parameters directory does not exist: {}",
        params_dir.display()
    );

    info!("⬇ Downloading proof parameters (this may take a while)...");

    // Try primary method: lotus fetch-params
    let primary_result = download_via_lotus_fetch_params(&params_dir);

    match primary_result {
        Ok(_) => {
            info!("✓ Proof parameters downloaded successfully via lotus fetch-params");
            return Ok(());
        }
        Err(e) => {
            warn!("Primary download method (lotus fetch-params) failed: {}", e);
            warn!("Falling back to S3 tarball download...");
        }
    }

    // Fallback: Download from S3
    download_from_s3(&params_dir)?;

    info!("✓ Proof parameters downloaded successfully via S3 fallback");
    Ok(())
}

/// Download proof parameters using lotus fetch-params.
///
/// This is the primary download method that uses the lotus binary's
/// built-in parameter fetching functionality.
fn download_via_lotus_fetch_params(
    params_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Retry the download operation in case of network issues
    retry_with_fixed_delay(
        || {
            // Ensure directory exists for each attempt (in case cleanup removed it)
            fs::create_dir_all(params_dir)?;

            // Run lotus fetch-params in builder container
            let bin_dir = foc_devnet_bin();
            let builder_volumes_dir = foc_devnet_docker_volumes().join("builder");

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
            let params_dir_clone = params_dir.to_path_buf();

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

            let container_name = format!(
                "foc-proof-params-fetch-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs()
            );
            let child = Command::new("docker")
                .args([
                    "run",
                    "--name",
                    &container_name,
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
                    crate::constants::BUILDER_DOCKER_IMAGE,
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
                    if let Err(cleanup_err) = fs::remove_dir_all(params_dir) {
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
    )
}

/// Download proof parameters from S3 as a fallback.
///
/// This method downloads a pre-packaged tarball of proof parameters from S3,
/// extracts it, and places the files in the correct location.
fn download_from_s3(params_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let tarball_path = std::env::temp_dir().join("filecoin-proof-params-2k.tar");

    // Ensure params directory exists
    fs::create_dir_all(params_dir)?;

    // Download tarball with retry
    retry_with_fixed_delay(
        || {
            info!("Downloading proof parameters tarball from S3...");

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap()
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
            );
            pb.set_message("Downloading tarball from S3...");

            let output = Command::new("curl")
                .args([
                    "-L",
                    PROOF_PARAMS_S3_URL,
                    "-o",
                    &tarball_path.to_string_lossy(),
                ])
                .output()?;

            pb.finish_and_clear();

            if !output.status.success() {
                // Clean up failed download
                if tarball_path.exists() {
                    let _ = fs::remove_file(&tarball_path);
                }
                return Err(format!(
                    "Failed to download tarball from S3: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }

            Ok(())
        },
        DEFAULT_MAX_RETRIES,
        DEFAULT_RETRY_DELAY_SECS,
        "S3 tarball download",
    )?;

    // Extract tarball
    info!("Extracting proof parameters tarball...");
    let extract_output = Command::new("tar")
        .args([
            "-xf",
            &tarball_path.to_string_lossy(),
            "-C",
            &params_dir.to_string_lossy(),
        ])
        .output()?;

    if !extract_output.status.success() {
        // Clean up on extraction failure
        let _ = fs::remove_file(&tarball_path);
        if params_dir.exists() {
            let _ = fs::remove_dir_all(params_dir);
        }
        return Err(format!(
            "Failed to extract tarball: {}",
            String::from_utf8_lossy(&extract_output.stderr)
        )
        .into());
    }

    // Clean up tarball after successful extraction
    if tarball_path.exists() {
        fs::remove_file(&tarball_path)?;
    }

    // Verify extraction succeeded by checking for files
    if !params_dir.exists() || params_dir.read_dir()?.next().is_none() {
        return Err("Tarball extraction produced no files".into());
    }

    info!("Proof parameters extracted successfully");
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
