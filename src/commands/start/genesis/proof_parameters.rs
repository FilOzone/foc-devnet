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
use std::fs;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Ensure Filecoin proof parameters are downloaded.
///
/// Parameters are downloaded once and cached in ~/.foc-localnet/artifacts/filecoin-proof-parameters/
/// This directory is mounted into lotus containers at /var/tmp/filecoin-proof-parameters/
pub fn ensure_proof_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = foc_localnet_proof_parameters();

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
