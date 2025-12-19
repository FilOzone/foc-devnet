//! Artifact download utilities for foc-localnet initialization.
//!
//! This module handles the downloading and extraction of required artifacts,
//! primarily the Yugabyte database.

use downloader::Downloader;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

use crate::config::Config;
use crate::paths::{foc_localnet_artifacts, foc_localnet_config, foc_localnet_proof_parameters};

/// Download required artifacts for foc-localnet.
///
/// This function downloads Yugabyte database and extracts it to the
/// artifacts directory, or copies from local paths if provided.
///
/// # Arguments
/// * `yugabyte_archive` - Optional path to local Yugabyte archive file
/// * `proof_params_dir` - Optional path to local filecoin-proof-params directory
///
/// # Returns
/// Returns `Ok(())` if artifacts are downloaded successfully, or an error if download fails.
pub fn download_artifacts(
    yugabyte_archive: Option<String>,
    proof_params_dir: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Downloading artifacts...");

    // Ensure artifacts directory exists
    let artifacts_dir = foc_localnet_artifacts();
    fs::create_dir_all(&artifacts_dir)?;

    // Handle Yugabyte
    if let Some(archive_path) = yugabyte_archive {
        copy_yugabyte_from_local(&archive_path, &artifacts_dir)?;
    } else {
        // Load configuration to get download URL
        let config_path = foc_localnet_config();
        let config_content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
        let config: Config = toml::from_str(&config_content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;
        download_yugabyte(&config.yugabyte_download_url, &artifacts_dir)?;
    }

    // Handle proof parameters
    if let Some(params_path) = proof_params_dir {
        copy_proof_params_from_local(&params_path)?;
    }

    info!("Artifacts downloaded successfully.");
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

    if tarball_path.exists() {
        info!(
            "Yugabyte tarball already exists at {}",
            tarball_path.display()
        );
        return Ok(tarball_path);
    }

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
    let yugabyte_dir = artifacts_dir.join("yugabyte");

    if yugabyte_dir.exists() {
        info!(
            "Yugabyte directory already exists at {}",
            yugabyte_dir.display()
        );
        return Ok(());
    }

    info!("Extracting Yugabyte tarball...");

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

    info!("Yugabyte downloaded and installed successfully.");

    Ok(())
}

/// Copy Yugabyte from a local archive file.
///
/// This function extracts a local Yugabyte tarball instead of downloading it.
///
/// # Arguments
/// * `archive_path` - Path to the local Yugabyte tarball
/// * `artifacts_dir` - Directory to extract Yugabyte into
///
/// # Returns
/// Returns `Ok(())` if copy and extraction succeed, or an error if they fail.
fn copy_yugabyte_from_local(
    archive_path: &str,
    artifacts_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let archive_path = Path::new(archive_path);
    if !archive_path.exists() {
        return Err(format!(
            "Local Yugabyte archive not found: {}",
            archive_path.display()
        )
        .into());
    }

    info!(
        "Copying Yugabyte from local archive: {}",
        archive_path.display()
    );

    extract_yugabyte_tarball(archive_path, artifacts_dir)?;
    Ok(())
}

/// Copy filecoin-proof-params from a local directory.
///
/// This function copies proof parameters from a local directory instead of downloading them.
///
/// # Arguments
/// * `params_path` - Path to the local proof parameters directory
///
/// # Returns
/// Returns `Ok(())` if copy succeeds, or an error if it fails.
fn copy_proof_params_from_local(params_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let params_path = Path::new(params_path);
    if !params_path.exists() {
        return Err(format!(
            "Local proof parameters not found: {}",
            params_path.display()
        )
        .into());
    }

    info!(
        "Copying proof parameters from local directory: {}",
        params_path.display()
    );

    let dest_path = foc_localnet_proof_parameters();

    // Create destination directory
    fs::create_dir_all(&dest_path)?;

    // Copy all files from source to destination
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Copying proof parameters...");

    copy_dir_recursive(params_path, &dest_path)?;

    pb.finish_with_message("✓ Proof parameters copied");

    info!("Proof parameters copied successfully");
    Ok(())
}

/// Recursively copy a directory.
///
/// # Arguments
/// * `src` - Source directory path
/// * `dst` - Destination directory path
///
/// # Returns
/// Returns `Ok(())` if copy succeeds, or an error if it fails.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}
