//! Sector pre-sealing for genesis preparation.
//!
//! This module handles pre-sealing sectors required for the genesis miners.

use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes_cache, foc_localnet_genesis,
    foc_localnet_genesis_sectors, foc_localnet_genesis_sectors_lotus_miner,
    foc_localnet_genesis_sectors_pdp_sp,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tracing::info;

/// Ensure sectors are pre-sealed for genesis.
///
/// Pre-seals sectors for lotus-miner and PDP SP miners using lotus-seed
/// and stores them in the genesis-sectors directory.
///
/// # Parameters
/// - `active_pdp_sp_count`: Number of active PDP SPs to create sectors for
/// - `run_id`: The run ID for this cluster
///
/// # Returns
/// Returns `Ok(())` if sectors exist or are successfully generated.
pub fn ensure_presealed_sectors(
    active_pdp_sp_count: usize,
    run_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let sectors_dir = foc_localnet_genesis_sectors(run_id);
    let genesis_dir = foc_localnet_genesis(run_id);

    // Build list of all miner directories to check: lotus-miner + PDP SPs
    let mut miner_dirs = vec![foc_localnet_genesis_sectors_lotus_miner(run_id)];

    // Add PDP SP directories
    for i in 1..=active_pdp_sp_count {
        miner_dirs.push(foc_localnet_genesis_sectors_pdp_sp(run_id, i));
    }

    // Check if sectors already exist for all miners
    let all_exist = miner_dirs.iter().all(|dir| {
        dir.exists()
            && dir
                .read_dir()
                .map(|mut rd| rd.next().is_some())
                .unwrap_or(false)
    });

    if all_exist {
        let total_miners = 1 + active_pdp_sp_count;
        info!(
            "✓ Pre-sealed sectors already exist for all {} miners",
            total_miners
        );
        return Ok(());
    }

    let total_miners = 1 + active_pdp_sp_count;
    info!(
        "⚙ Pre-sealing {} sectors (size: {}) for {} miners...",
        total_miners,
        super::constants::SECTOR_SIZE,
        total_miners
    );

    // Create the directories
    fs::create_dir_all(&sectors_dir)?;
    fs::create_dir_all(&genesis_dir)?;

    // Pre-seal sectors for lotus-miner
    let mut miner_configs: Vec<(String, PathBuf)> = vec![(
        super::constants::LOTUS_MINER_ID.to_string(),
        foc_localnet_genesis_sectors_lotus_miner(run_id),
    )];

    // Add PDP SP miners
    for i in 1..=active_pdp_sp_count {
        let miner_id = format!(
            "t0{}",
            super::constants::PDP_SP_MINER_ID_START + (i as u32) - 1
        );
        let miner_dir = foc_localnet_genesis_sectors_pdp_sp(run_id, i);
        miner_configs.push((miner_id, miner_dir));
    }

    for (miner_id, miner_dir) in miner_configs {
        preseal_miner_sectors(&miner_id, &miner_dir, run_id)?;
    }

    info!(
        "✓ Sectors pre-sealed successfully for all {} miners",
        total_miners
    );
    Ok(())
}

/// Pre-seal sectors for a single miner.
fn preseal_miner_sectors(
    miner_id: &str,
    miner_dir: &PathBuf,
    run_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("⚙ Pre-sealing sectors for miner {}...", miner_id,);

    // Create miner directory
    fs::create_dir_all(miner_dir)?;

    // Run lotus-seed pre-seal in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir =
        foc_localnet_docker_volumes_cache().join(crate::constants::BUILDER_CONTAINER);

    // Build docker args with network environment variables
    let mut docker_args = vec![
        "run".to_string(),
        "-u".to_string(),
        "foc-user".to_string(),
        "--name".to_string(),
        format!("foc-{}-genesis-preseal-{}", run_id, miner_id),
    ];

    // Add volume mounts and command
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
        crate::constants::BUILDER_DOCKER_IMAGE.to_string(),
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

    Ok(())
}
