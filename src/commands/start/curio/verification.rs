//! Verification tests for Curio PDP functionality.
//!
//! Tests:
//! - PDP subsystem ping
//! - File upload via pdptool
//! - File download and content verification

use super::super::step::SetupContext;
use super::constants::TEST_FILE_SIZE_BYTES;
use rand::Rng;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use tracing::info;

/// Verify a single Curio PDP SP is functioning correctly.
///
/// Checks:
/// 1. PDP subsystem responds to ping
/// 2. Can upload a test file via pdptool
/// 3. Can download the file and verify contents match
#[allow(unused_variables)]
pub fn verify_single_curio_sp(
    context: &SetupContext,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Ping PDP subsystem
    verify_pdp_ping(context, sp_index)?;

    // Step 2: Upload and download test
    verify_upload_download(context, sp_index)?;

    Ok(())
}

/// Verify PDP subsystem responds to ping.
fn verify_pdp_ping(context: &SetupContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    info!("Pinging PDP subsystem...");

    // Get dynamically allocated PDP port from context
    let port: u16 = context
        .get(&format!("curio_sp_{}_pdp_port", sp_index))
        .ok_or("Curio PDP port not found in context")?
        .parse()?;

    let ping_url = format!("http://localhost:{}/pdp/ping", port);

    let response = reqwest::blocking::get(&ping_url)?;

    if !response.status().is_success() {
        return Err(format!("PDP ping failed with status: {}", response.status()).into());
    }

    info!("PDP subsystem responding");

    Ok(())
}

/// Verify file upload and download works correctly.
fn verify_upload_download(context: &SetupContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    info!("Testing upload/download functionality via pdptool...");

    // Create test file in curio's fast-storage (already mounted to container)
    let run_id = context.run_id();
    let curio_sp_dir = crate::paths::foc_devnet_curio_sp_volume(run_id, sp_index);
    let test_file_dir = curio_sp_dir.join("fast-storage");
    fs::create_dir_all(&test_file_dir)?;

    let test_file_path = create_random_test_file(&test_file_dir)?;

    // Upload file via pdptool (running in container)
    let piece_cid = upload_test_file(context, &test_file_path, sp_index)?;

    // Wait a bit for the piece to be available for download
    sleep(Duration::from_secs(3));

    // Download file via HTTP
    let downloaded_data = download_piece(context, &piece_cid, sp_index)?;

    // Verify contents match
    let original_data = fs::read(&test_file_path)?;
    if original_data != downloaded_data {
        return Err("Downloaded data does not match original".into());
    }

    // Clean up test file
    let _ = fs::remove_file(&test_file_path);

    info!("Upload/download verified");

    Ok(())
}

/// Create a random test file.
fn create_random_test_file(test_dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let test_file_path = test_dir.join("test_data.bin");
    let mut rng = rand::thread_rng();
    let random_data: Vec<u8> = (0..TEST_FILE_SIZE_BYTES).map(|_| rng.gen()).collect();

    fs::write(&test_file_path, random_data)?;

    Ok(test_file_path)
}

/// Upload test file using pdptool.
fn upload_test_file(
    context: &SetupContext,
    _file_path: &Path,
    sp_index: usize,
) -> Result<String, Box<dyn Error>> {
    // When running pdptool inside the container, use the container's internal port (4702)
    // not the host-mapped port
    let service_url = "http://localhost:4702";

    // File is in fast-storage on host, which is mounted to /home/foc-user/curio/fast-storage in container
    let container_file_path = "/home/foc-user/curio/fast-storage/test_data.bin";

    let run_id = context.run_id();
    let container_name = format!("foc-{}-curio-{}", run_id, sp_index);

    let args = [
        "exec",
        &container_name,
        "/usr/local/bin/lotus-bins/pdptool",
        "upload-piece",
        "--service-url",
        &service_url,
        "--service-name",
        "public",
        "--hash-type",
        "commp",
        container_file_path,
        "--verbose",
    ];

    let output = Command::new("docker")
        .args(args)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "pdptool upload failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    info!("File uploaded via pdptool");

    // Extract piece CID from output
    let stdout = String::from_utf8_lossy(&output.stdout);
    extract_piece_cid(&stdout)
}

/// Extract piece CID from pdptool output.
///
/// Parses output like: "Piece uploaded successfully. Piece CID: baga6ea4seaq..."
fn extract_piece_cid(output: &str) -> Result<String, Box<dyn Error>> {
    for line in output.lines() {
        if let Some(prefix_pos) = line.find("Piece CID:") {
            // Extract everything after "Piece CID: "
            let cid_part = &line[prefix_pos + "Piece CID:".len()..];
            let cid = cid_part.trim();

            info!("Extracted Piece CID: {}", cid);
            if !cid.is_empty() {
                return Ok(cid.to_string());
            }
        }
    }

    Err(format!(
        "Could not extract piece CID from pdptool output. Output was:\n{}",
        output
    )
    .into())
}

/// Download piece via HTTP.
fn download_piece(
    context: &SetupContext,
    piece_cid: &str,
    sp_index: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    // Get dynamically allocated PDP port from context
    let port: u16 = context
        .get(&format!("curio_sp_{}_pdp_port", sp_index))
        .ok_or("Curio PDP port not found in context")?
        .parse()?;

    let download_url = format!("http://localhost:{}/piece/{}", port, piece_cid);

    // Retry download a few times in case piece isn't immediately available
    for attempt in 1..=15 {
        let response = reqwest::blocking::get(&download_url)?;

        if response.status().is_success() {
            let data = response.bytes()?.to_vec();
            return Ok(data);
        }

        if attempt < 15 {
            info!(
                "Download attempt {} failed with status: {}, retrying...",
                attempt,
                response.status()
            );
            sleep(Duration::from_secs(4));
        }
    }

    // Final attempt
    let response = reqwest::blocking::get(&download_url)?;
    if !response.status().is_success() {
        return Err(format!("Piece download failed with status: {}", response.status()).into());
    }

    let data = response.bytes()?.to_vec();
    Ok(data)
}
