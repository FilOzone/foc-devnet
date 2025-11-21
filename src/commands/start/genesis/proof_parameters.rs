//! Proof parameters management for genesis preparation.
//!
//! This module handles downloading and caching Filecoin proof parameters
//! required for lotus operations.

use crate::paths::{
    CONTAINER_FILECOIN_PROOF_PARAMS_PATH, foc_localnet_bin, foc_localnet_docker_volumes,
    foc_localnet_proof_parameters,
};
use crossterm::style::Stylize;
use std::fs;
use std::process::Command;

/// Ensure Filecoin proof parameters are downloaded.
///
/// Parameters are downloaded once and cached in ~/.foc-localnet/artifacts/filecoin-proof-parameters/
/// This directory is mounted into lotus containers at /var/tmp/filecoin-proof-parameters/
pub fn ensure_proof_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = foc_localnet_proof_parameters();

    // Check if parameters already exist
    if params_dir.exists() && params_dir.read_dir()?.next().is_some() {
        println!(
            "  {} Proof parameters already exist at {}",
            "✓".green(),
            params_dir.display()
        );
        return Ok(());
    }

    println!(
        "  {} Downloading proof parameters (this may take a while)...",
        "⬇".cyan()
    );

    // Create the directory
    fs::create_dir_all(&params_dir)?;

    // Run lotus fetch-params in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = Command::new("docker")
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
                "{}:/root/.cargo",
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
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to download proof parameters: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Proof parameters downloaded successfully", "✓".green());
    Ok(())
}
