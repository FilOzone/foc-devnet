//! Genesis miner management.
//!
//! This module handles adding pre-sealed miners to the genesis file.

use crate::commands::start::env_vars::build_network_env_vars;
use crate::commands::start::genesis::constants;
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis,
    foc_localnet_genesis_sectors_curio_miner, foc_localnet_genesis_sectors_lotus_miner,
    foc_localnet_genesis_sectors_pdp_sp,
};
use crossterm::style::Stylize;
use std::path::PathBuf;
use std::process::Command;

/// Add miners to the genesis file.
///
/// Runs `lotus-seed genesis add-miner` to add the pre-sealed miners
/// to the genesis configuration.
pub fn add_miner_to_genesis() -> Result<(), Box<dyn std::error::Error>> {
    println!("  {} Adding miners to genesis...", "⛏".cyan());

    // Build list of all miners to add
    let mut miner_configs: Vec<(String, PathBuf)> = vec![
        (constants::LOTUS_MINER_ID.to_string(), foc_localnet_genesis_sectors_lotus_miner()),
        (constants::CURIO_MINER_ID.to_string(), foc_localnet_genesis_sectors_curio_miner()),
    ];

    // Add PDP SP miners
    for i in 1..=constants::ACTIVE_PDP_SP_COUNT {
        let miner_id = format!("t0{}", constants::PDP_SP_MINER_ID_START + (i as u32) - 1);
        let miner_dir = foc_localnet_genesis_sectors_pdp_sp(i);
        miner_configs.push((miner_id, miner_dir));
    }

    for (miner_id, miner_dir) in miner_configs {
        add_single_miner_to_genesis(&miner_id, &miner_dir)?;
    }

    let total_miners = 2 + constants::ACTIVE_PDP_SP_COUNT;
    println!(
        "  {} All {} miners added to genesis successfully",
        "✓".green(),
        total_miners
    );
    Ok(())
}

/// Add a single miner to the genesis file.
fn add_single_miner_to_genesis(miner_id: &str, miner_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
    let genesis_dir = foc_localnet_genesis();
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

    Ok(())
}
