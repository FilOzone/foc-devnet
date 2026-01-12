//! Genesis miner management.
//!
//! This module handles adding pre-sealed miners to the genesis file.

use crate::commands::start::genesis::constants;
use crate::paths::{
    foc_devnet_bin, foc_devnet_docker_volumes_cache, foc_devnet_genesis,
    foc_devnet_genesis_sectors_lotus_miner, foc_devnet_genesis_sectors_pdp_sp,
};
use std::path::PathBuf;
use std::process::Command;
use tracing::info;

/// Add miners to the genesis file.
///
/// Runs `lotus-seed genesis add-miner` to add the pre-sealed miners
/// to the genesis configuration.
///
/// # Parameters
/// - `active_pdp_sp_count`: Number of active PDP SPs to add to genesis
/// - `run_id`: The run ID for this cluster
///
/// # Returns
/// Returns `Ok(())` if all miners are added successfully.
pub fn add_miner_to_genesis(
    active_pdp_sp_count: usize,
    run_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("⛏ Adding miners to genesis...");

    // Build list of all miners to add: lotus-miner + PDP SPs
    let mut miner_configs: Vec<(String, PathBuf)> = vec![(
        constants::LOTUS_MINER_ID.to_string(),
        foc_devnet_genesis_sectors_lotus_miner(run_id),
    )];

    // Add PDP SP miners
    for i in 1..=active_pdp_sp_count {
        let miner_id = format!("t0{}", constants::PDP_SP_MINER_ID_START + (i as u32) - 1);
        let miner_dir = foc_devnet_genesis_sectors_pdp_sp(run_id, i);
        miner_configs.push((miner_id, miner_dir));
    }

    for (miner_id, miner_dir) in miner_configs {
        add_single_miner_to_genesis(&miner_id, &miner_dir, run_id)?;
    }

    let total_miners = 1 + active_pdp_sp_count;
    info!(
        "✓ All {} miners added to genesis successfully",
        total_miners
    );
    Ok(())
}

/// Add a single miner to the genesis file.
fn add_single_miner_to_genesis(
    miner_id: &str,
    miner_dir: &PathBuf,
    run_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("⛏ Adding miner {} to genesis...", miner_id,);

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
    let genesis_dir = foc_devnet_genesis(run_id);
    let bin_dir = foc_devnet_bin();
    let builder_volumes_dir =
        foc_devnet_docker_volumes_cache().join(crate::constants::BUILDER_CONTAINER);

    // Build docker args with network environment variables
    let mut docker_args = vec![
        "run".to_string(),
        "-u".to_string(),
        "foc-user".to_string(),
        "--name".to_string(),
        format!("foc-{}-genesis-add-miner-{}", run_id, miner_id),
    ];

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
        crate::constants::BUILDER_DOCKER_IMAGE.to_string(),
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
