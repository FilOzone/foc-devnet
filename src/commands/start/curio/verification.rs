//! Verification tests for Curio PDP functionality.
//!
//! Tests:
//! - PDP subsystem ping
//! - File upload via pdptool
//! - File download and content verification

use super::super::step::StepContext;
use super::constants::TEST_FILE_SIZE_BYTES;
use crossterm::style::Stylize;
use rand::Rng;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
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
    verify_pdp_ping(context, sp_index)?;

    // Step 2: Upload and download test
    verify_upload_download(context, sp_index)?;

    Ok(())
}

/// Verify PDP subsystem responds to ping.
fn verify_pdp_ping(context: &StepContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!("      {} Pinging PDP subsystem...", "🏓".cyan());

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

    println!("      {} PDP subsystem responding", "✓".green());

    Ok(())
}

/// Verify file upload and download works correctly.
fn verify_upload_download(context: &StepContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!(
        "      {} Testing upload/download functionality...",
        "📤".cyan()
    );

    // Create temporary directory for test files
    let temp_dir = TempDir::new()?;
    let test_file_path = create_random_test_file(&temp_dir)?;

    // Upload file via pdptool
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
fn upload_test_file(
    context: &StepContext,
    file_path: &PathBuf,
    sp_index: usize,
) -> Result<String, Box<dyn Error>> {
    // Get dynamically allocated PDP port from context
    let port: u16 = context
        .get(&format!("curio_sp_{}_pdp_port", sp_index))
        .ok_or("Curio PDP port not found in context")?
        .parse()?;

    let service_url = format!("http://localhost:{}", port);

    // Run pdptool upload twice due to tool quirk
    let args = [
        "upload-piece",
        "--service-url",
        &service_url,
        "--service-name",
        "public",
        "--hash-type",
        "commp",
        file_path.to_str().unwrap(),
        "--verbose",
    ];

    let output = Command::new("pdptool").args(args).output()?;

    if !output.status.success() {
        return Err(format!(
            "pdptool upload failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("      {} File uploaded via pdptool", "✓".green(),);

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

            println!("      {} Extracted Piece CID: {}", "🔍".cyan(), cid);
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
    context: &StepContext,
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
    for attempt in 1..=5 {
        let response = reqwest::blocking::get(&download_url)?;

        if response.status().is_success() {
            let data = response.bytes()?.to_vec();
            return Ok(data);
        }

        if attempt < 5 {
            println!(
                "      {} Download attempt {} failed with status: {}, retrying...",
                "⏳".yellow(),
                attempt,
                response.status()
            );
            sleep(Duration::from_secs(2));
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
