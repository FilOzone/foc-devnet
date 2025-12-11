//! Sector pre-sealing for genesis preparation.
//!
//! This module handles pre-sealing sectors required for the genesis miners.

use crate::commands::start::env_vars::build_network_env_vars;
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis,
    foc_localnet_genesis_sectors, foc_localnet_genesis_sectors_curio_miner,
    foc_localnet_genesis_sectors_lotus_miner,
};
use crossterm::style::Stylize;
use std::fs;
use std::process::Command;

/// Ensure sectors are pre-sealed for genesis.
///
/// Pre-seals sectors for two miners using lotus-seed and stores them in the genesis-sectors directory.
pub fn ensure_presealed_sectors() -> Result<(), Box<dyn std::error::Error>> {
    let sectors_dir = foc_localnet_genesis_sectors();
    let miner_1_dir = foc_localnet_genesis_sectors_lotus_miner();
    let miner_2_dir = foc_localnet_genesis_sectors_curio_miner();
    let genesis_dir = foc_localnet_genesis();

    // Check if sectors already exist
    if miner_1_dir.exists()
        && miner_1_dir.read_dir()?.next().is_some()
        && miner_2_dir.exists()
        && miner_2_dir.read_dir()?.next().is_some()
    {
        println!(
            "  {} Pre-sealed sectors already exist for both miners",
            "✓".green()
        );
        return Ok(());
    }

    println!(
        "  {} Pre-sealing {} sectors (size: {}) for {} miners...",
        "⚙".cyan(),
        super::constants::NUM_SECTORS,
        super::constants::SECTOR_SIZE,
        2
    );

    // Create the directories
    fs::create_dir_all(&sectors_dir)?;
    fs::create_dir_all(&miner_1_dir)?;
    fs::create_dir_all(&miner_2_dir)?;
    fs::create_dir_all(&genesis_dir)?;

    // Pre-seal sectors for each miner in their respective directories
    let miner_configs = vec![
        (super::constants::LOTUS_MINER_ID, &miner_1_dir),
        (super::constants::CURIO_MINER_ID, &miner_2_dir),
    ];

    for (miner_id, miner_dir) in miner_configs {
        println!(
            "    {} Pre-sealing sectors for miner {} in {}",
            "⛏".cyan(),
            miner_id,
            miner_dir.display()
        );

        // Pre-seal sectors using lotus-seed in builder container
        let bin_dir = foc_localnet_bin();
        let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

        // Build docker args with network environment variables
        let mut docker_args = vec!["run".to_string(), "--rm".to_string()];

        // Add network environment variables (required for lotus-seed built with -tags=localnet)
        docker_args.extend(build_network_env_vars());

        // Add volume mounts
        docker_args.extend(vec![
            "-v".to_string(),
            format!("{}:/opt/bin", bin_dir.display()),
            "-v".to_string(),
            format!(
                "{}:/home/foc-user/.cargo",
                builder_volumes_dir.join("cargo").display()
            ),
            "-v".to_string(),
            format!("{}:/home/foc-user/.genesis-sectors", miner_dir.display()),
            "foc-builder".to_string(),
            "/bin/bash".to_string(),
            "-c".to_string(),
            format!(
                "/opt/bin/lotus-seed pre-seal --sector-size {} --num-sectors {} --miner-addr {}",
                super::constants::SECTOR_SIZE,
                super::constants::NUM_SECTORS,
                miner_id
            ),
        ]);

        let output = Command::new("docker").args(&docker_args).output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to pre-seal sectors for miner {}: {}",
                miner_id,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
    }

    println!(
        "  {} Sectors pre-sealed successfully for both miners",
        "✓".green()
    );
    Ok(())
}
