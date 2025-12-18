//! Genesis file creation.
//!
//! This module handles creating the initial genesis file using lotus-seed.

use crate::commands::start::genesis::constants;
use crate::paths::{foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis};
use crossterm::style::Stylize;
use std::fs;
use std::process::Command;

/// Create the initial genesis file.
///
/// Runs `lotus-seed genesis new` to create a new genesis file with the network name
/// and current timestamp.
///
/// Note: This function assumes the genesis file does not already exist.
/// The caller should check for existence first.
pub fn create_genesis_file() -> Result<(), Box<dyn std::error::Error>> {
    let genesis_dir = foc_localnet_genesis();

    println!("  {} Creating genesis file...", "📜".cyan());

    // Ensure genesis directory exists
    fs::create_dir_all(&genesis_dir)?;

    // Get current timestamp in ISO 8601 format (RFC3339)
    // lotus-seed expects format like: 2006-01-02T15:04:05Z
    let now = std::time::SystemTime::now();
    let datetime: chrono::DateTime<chrono::Utc> = now.into();
    let timestamp = datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Run lotus-seed genesis new in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    // Build docker args with network environment variables
    let mut docker_args = vec!["run".to_string(), "--rm".to_string()];

    // Add volume mounts and command
    docker_args.extend(vec![
        "-v".to_string(),
        format!("{}:/opt/bin", bin_dir.display()),
        "-v".to_string(),
        format!("{}:/home/foc-user/.cargo", builder_volumes_dir.join("cargo").display()),
        "-v".to_string(),
        format!("{}:/genesis", genesis_dir.display()),
        "foc-builder".to_string(),
        "/bin/bash".to_string(),
        "-c".to_string(),
        format!(
            "/opt/bin/lotus-seed genesis new --network-name {} --timestamp {} /genesis/{} && chmod 666 /genesis/{}",
            constants::NETWORK_NAME, timestamp, constants::GENESIS_FILE, constants::GENESIS_FILE
        ),
    ]);

    let output = Command::new("docker").args(&docker_args).output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create genesis file: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Genesis file created successfully", "✓".green());
    Ok(())
}
