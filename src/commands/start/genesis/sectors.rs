//! Sector pre-sealing for genesis preparation.
//!
//! This module handles pre-sealing sectors required for the genesis miner.

use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis,
    foc_localnet_genesis_sectors,
};
use crossterm::style::Stylize;
use std::fs;
use std::process::Command;

/// Ensure sectors are pre-sealed for genesis.
///
/// Pre-seals 2 sectors using lotus-seed and stores them in the genesis-sectors directory.
pub fn ensure_presealed_sectors() -> Result<(), Box<dyn std::error::Error>> {
    let sectors_dir = foc_localnet_genesis_sectors();
    let genesis_dir = foc_localnet_genesis();

    // Check if sectors already exist
    if sectors_dir.exists() && sectors_dir.read_dir()?.next().is_some() {
        println!(
            "  {} Pre-sealed sectors already exist at {}",
            "✓".green(),
            sectors_dir.display()
        );
        return Ok(());
    }

    println!(
        "  {} Pre-sealing {} sectors (size: {})...",
        "⚙".cyan(),
        super::constants::NUM_SECTORS,
        super::constants::SECTOR_SIZE
    );

    // Create the directories
    fs::create_dir_all(&sectors_dir)?;
    fs::create_dir_all(&genesis_dir)?;

    // Pre-seal sectors using lotus-seed in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/opt/bin", bin_dir.display()),
            "-v",
            &format!(
                "{}:/root/.cargo",
                builder_volumes_dir.join("cargo").display()
            ),
            "-v",
            &format!("{}:/root/.genesis-sectors", sectors_dir.display()),
            "foc-builder",
            "/bin/bash",
            "-c",
            &format!(
                "/opt/bin/lotus-seed pre-seal --sector-size {} --num-sectors {}",
                super::constants::SECTOR_SIZE,
                super::constants::NUM_SECTORS
            ),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to pre-seal sectors: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Sectors pre-sealed successfully", "✓".green());
    Ok(())
}
