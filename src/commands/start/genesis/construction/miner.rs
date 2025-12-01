//! Genesis miner management.
//!
//! This module handles adding pre-sealed miners to the genesis file.

use crate::commands::start::genesis::constants;
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis,
    foc_localnet_genesis_sectors,
};
use crossterm::style::Stylize;
use std::process::Command;

/// Add a miner to the genesis file.
///
/// Runs `lotus-seed genesis add-miner` to add the pre-sealed miner (t01000)
/// to the genesis configuration.
pub fn add_miner_to_genesis() -> Result<(), Box<dyn std::error::Error>> {
    println!("  {} Adding miner to genesis...", "⛏".cyan());

    let genesis_dir = foc_localnet_genesis();
    let sectors_dir = foc_localnet_genesis_sectors();

    // Check for pre-seal file (typically pre-seal-t01000.json)
    let preseal_file = sectors_dir.join("pre-seal-t01000.json");

    if !preseal_file.exists() {
        return Err(format!(
            "Pre-seal file not found at {}. Ensure sectors are pre-sealed first.",
            preseal_file.display()
        )
        .into());
    }

    // Run lotus-seed genesis add-miner in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/opt/bin", bin_dir.display()),
            "-v",
            &format!("{}:/home/foc-user/.cargo", builder_volumes_dir.join("cargo").display()),
            "-v",
            &format!("{}:/genesis", genesis_dir.display()),
            "-v",
            &format!("{}:/home/foc-user/.genesis-sectors", sectors_dir.display()),
            "foc-builder",
            "/bin/bash",
            "-c",
            &format!(
                "/opt/bin/lotus-seed genesis add-miner /genesis/{} /home/foc-user/.genesis-sectors/pre-seal-t01000.json",
                constants::GENESIS_FILE
            ),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to add miner to genesis: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Miner added to genesis successfully", "✓".green());
    Ok(())
}
