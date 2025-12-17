//! Verification tests for Curio PDP functionality.
//!
//! Tests:
//! - PDP subsystem ping
//! - File upload via pdptool
//! - File download and content verification

use super::super::step::StepContext;
use super::constants::{CURIO_WEB_RPC_PORT, TEST_FILE_SIZE_BYTES};
use crossterm::style::Stylize;
use rand::Rng;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Verify a single Curio PDP SP is functioning correctly.
///
/// Checks:
/// 1. PDP subsystem responds to ping
/// 2. Can upload a test file via pdptool
/// 3. Can download the file and verify contents match
#[allow(unused_variables)]
pub fn verify_single_curio_sp(
    context: &StepContext,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Ping PDP subsystem
    verify_pdp_ping(sp_index)?;

    // Step 2: Upload and download test
    verify_upload_download(sp_index)?;

    Ok(())
}

/// Verify PDP subsystem responds to ping.
fn verify_pdp_ping(sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!("      {} Pinging PDP subsystem...", "🏓".cyan());

    let port = CURIO_WEB_RPC_PORT + sp_index as u16;
    let ping_url = format!("http://localhost:{}/pdp/ping", port);

    let response = reqwest::blocking::get(&ping_url)?;

    if !response.status().is_success() {
        return Err(format!("PDP ping failed with status: {}", response.status()).into());
    }

    println!("      {} PDP subsystem responding", "✓".green());

    Ok(())
}

/// Verify file upload and download works correctly.
fn verify_upload_download(sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!(
        "      {} Testing upload/download functionality...",
        "📤".cyan()
    );

    // Create temporary directory for test files
    let temp_dir = TempDir::new()?;
    let test_file_path = create_random_test_file(&temp_dir)?;

    // Upload file via pdptool
    let piece_cid = upload_test_file(&test_file_path, sp_index)?;

    // Download file via HTTP
    let downloaded_data = download_piece(&piece_cid, sp_index)?;

    // Verify contents match
    let original_data = fs::read(&test_file_path)?;
    if original_data != downloaded_data {
        return Err("Downloaded data does not match original".into());
    }

    println!("      {} Upload/download verified", "✓".green());

    Ok(())
}

/// Create a random test file.
fn create_random_test_file(temp_dir: &TempDir) -> Result<PathBuf, Box<dyn Error>> {
    let test_file_path = temp_dir.path().join("test_data.bin");
    let mut rng = rand::thread_rng();
    let random_data: Vec<u8> = (0..TEST_FILE_SIZE_BYTES).map(|_| rng.gen()).collect();

    fs::write(&test_file_path, random_data)?;

    Ok(test_file_path)
}

/// Upload test file using pdptool.
fn upload_test_file(file_path: &PathBuf, sp_index: usize) -> Result<String, Box<dyn Error>> {
    let port = CURIO_WEB_RPC_PORT + sp_index as u16;
    let service_url = format!("http://localhost:{}", port);

    let output = Command::new("pdptool")
        .args([
            "upload-piece",
            "--service-url",
            &service_url,
            "--service-name",
            "public",
            "--hash-type",
            "commp",
            file_path.to_str().unwrap(),
            "--verbose",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "pdptool upload failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Extract piece CID from output
    let stdout = String::from_utf8_lossy(&output.stdout);
    extract_piece_cid(&stdout)
}

/// Extract piece CID from pdptool output.
fn extract_piece_cid(output: &str) -> Result<String, Box<dyn Error>> {
    // Look for line like: "Piece CID: baga6ea4seaq..."
    for line in output.lines() {
        if line.contains("Piece CID:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(cid) = parts.last() {
                return Ok(cid.to_string());
            }
        }
    }

    Err("Could not extract piece CID from pdptool output".into())
}

/// Download piece via HTTP.
fn download_piece(piece_cid: &str, sp_index: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let port = CURIO_WEB_RPC_PORT + sp_index as u16;
    let download_url = format!("http://localhost:{}/piece/{}", port, piece_cid);

    let response = reqwest::blocking::get(&download_url)?;

    if !response.status().is_success() {
        return Err(format!("Piece download failed with status: {}", response.status()).into());
    }

    let data = response.bytes()?.to_vec();

    Ok(data)
}
