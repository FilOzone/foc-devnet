//! Artifact download utilities for foc-localnet initialization.
//!
//! This module handles the downloading and extraction of required artifacts,
//! primarily the Yugabyte database.

use crossterm::style::Stylize;
use downloader::Downloader;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;
use crate::paths::{foc_localnet_artifacts, foc_localnet_config};

/// Download required artifacts for foc-localnet.
///
/// This function downloads Yugabyte database and extracts it to the
/// artifacts directory. It reads the download URL from the configuration.
///
/// # Returns
/// Returns `Ok(())` if artifacts are downloaded successfully, or an error if download fails.
pub fn download_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Downloading artifacts...".bold());

    // Load configuration
    let config_path = foc_localnet_config();
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Ensure artifacts directory exists
    let artifacts_dir = foc_localnet_artifacts();
    fs::create_dir_all(&artifacts_dir)?;

    // Download Yugabyte
    download_yugabyte(&config.yugabyte_download_url, &artifacts_dir)?;

    println!("  {} Artifacts downloaded successfully.", "✓".green());
    Ok(())
}

/// Downloads and extracts Yugabyte database.
///
/// Downloads the Yugabyte tarball from the given URL and extracts it
/// to the specified directory.
///
/// # Arguments
/// * `url` - Download URL for the Yugabyte tarball
/// * `artifacts_dir` - Directory to extract Yugabyte into
///
/// # Returns
/// Returns `Ok(())` if download and extraction succeed, or an error if they fail.
fn download_yugabyte(url: &str, artifacts_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let tarball_path = download_yugabyte_tarball(url, artifacts_dir)?;
    extract_yugabyte_tarball(&tarball_path, artifacts_dir)?;
    Ok(())
}

/// Download the Yugabyte tarball from the given URL.
///
/// # Arguments
/// * `url` - Download URL for the tarball
/// * `artifacts_dir` - Directory to save the tarball in
///
/// # Returns
/// Returns the path to the downloaded tarball, or an error if download fails.
fn download_yugabyte_tarball(
    url: &str,
    artifacts_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Extract filename from URL
    let filename = url
        .split('/')
        .next_back()
        .ok_or("Invalid URL: no filename")?;
    let tarball_path = artifacts_dir.join(filename);

    // Create progress bar
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Downloading Yugabyte from {}...", url));

    // Download the tarball using downloader
    let mut downloader = Downloader::builder()
        .download_folder(artifacts_dir)
        .build()?;

    let dl = downloader::Download::new(url).file_name(Path::new(&filename));
    downloader.download(&[dl])?;

    pb.finish_with_message("✓ Downloaded Yugabyte");

    Ok(tarball_path)
}

/// Extract the Yugabyte tarball to the artifacts directory.
///
/// This function extracts the tarball and renames the extracted directory
/// to "yugabyte" for consistency. If the yugabyte directory already exists,
/// extraction is skipped.
///
/// # Arguments
/// * `tarball_path` - Path to the downloaded tarball
/// * `artifacts_dir` - Directory containing the tarball and extraction target
///
/// # Returns
/// Returns `Ok(())` if extraction succeeds or is skipped, or an error if extraction fails.
fn extract_yugabyte_tarball(
    tarball_path: &Path,
    artifacts_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if yugabyte directory already exists
    let yugabyte_dir = artifacts_dir.join("yugabyte");
    if yugabyte_dir.exists() {
        println!(
            "  {} Yugabyte directory already exists, skipping extraction",
            "✓".green()
        );
        return Ok(());
    }

    // Extract the tarball
    let pb_extract = ProgressBar::new_spinner();
    pb_extract.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb_extract.set_message("Extracting Yugabyte...");

    let status = Command::new("tar")
        .args(["xfz", &tarball_path.to_string_lossy()])
        .current_dir(artifacts_dir)
        .status()?;

    if !status.success() {
        pb_extract.finish_with_message("❌ Failed to extract Yugabyte");
        return Err("Failed to extract Yugabyte tarball".to_string().into());
    }

    // Find the extracted directory and rename it to "yugabyte"
    let mut extracted_dir = None;
    for entry in fs::read_dir(artifacts_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("yugabyte-") {
                    extracted_dir = Some(path);
                    break;
                }
            }
        }
    }

    if let Some(extracted) = extracted_dir {
        fs::rename(&extracted, &yugabyte_dir)?;
    } else {
        return Err("Could not find extracted yugabyte directory".into());
    }

    pb_extract.finish_with_message("✓ Extracted Yugabyte");

    println!(
        "  {} Yugabyte downloaded and installed successfully.",
        "✓".green()
    );
    Ok(())
}
