//! Genesis miner management.
//!
//! This module handles adding pre-sealed miners to the genesis file.

use crate::commands::start::env_vars::build_network_env_vars;
use crate::commands::start::genesis::constants;
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis,
    foc_localnet_genesis_sectors_curio_miner, foc_localnet_genesis_sectors_lotus_miner,
};
use crossterm::style::Stylize;
use std::process::Command;

/// Add miners to the genesis file.
///
/// Runs `lotus-seed genesis add-miner` to add the pre-sealed miners
/// to the genesis configuration.
pub fn add_miner_to_genesis() -> Result<(), Box<dyn std::error::Error>> {
    println!("  {} Adding miners to genesis...", "⛏".cyan());

    let genesis_dir = foc_localnet_genesis();
    let miner_1_dir = foc_localnet_genesis_sectors_lotus_miner();
    let miner_2_dir = foc_localnet_genesis_sectors_curio_miner();

    // Add each miner to genesis from their respective directories
    let miner_configs = vec![
        (constants::LOTUS_MINER_ID, &miner_1_dir),
        (constants::CURIO_MINER_ID, &miner_2_dir),
    ];

    for (miner_id, miner_dir) in miner_configs {
        println!(
            "    {} Adding miner {} to genesis from {}...",
            "⛏".cyan(),
            miner_id,
            miner_dir.display()
        );

        // Check for pre-seal file (e.g., pre-seal-t01000.json)
        let preseal_file = miner_dir.join(format!("pre-seal-{}.json", miner_id));

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

        // Build docker args with network environment variables
        let mut docker_args = vec!["run".to_string(), "--rm".to_string()];

        // Add network environment variables (required for lotus-seed built with -tags=localnet)
        docker_args.extend(build_network_env_vars());

        // Add volume mounts and command
        docker_args.extend(vec![
            "-v".to_string(),
            format!("{}:/opt/bin", bin_dir.display()),
            "-v".to_string(),
            format!("{}:/home/foc-user/.cargo", builder_volumes_dir.join("cargo").display()),
            "-v".to_string(),
            format!("{}:/genesis", genesis_dir.display()),
            "-v".to_string(),
            format!("{}:/home/foc-user/.genesis-sectors", miner_dir.display()),
            "foc-builder".to_string(),
            "/bin/bash".to_string(),
            "-c".to_string(),
            format!(
                "/opt/bin/lotus-seed genesis add-miner /genesis/{} /home/foc-user/.genesis-sectors/pre-seal-{}.json",
                constants::GENESIS_FILE,
                miner_id
            ),
        ]);

        let output = Command::new("docker").args(&docker_args).output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to add miner {} to genesis: {}",
                miner_id,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
    }

    println!("  {} Miners added to genesis successfully", "✓".green());
    Ok(())
}
