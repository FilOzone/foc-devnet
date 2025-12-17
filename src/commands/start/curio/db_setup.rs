//! Database setup for Curio PDP Service Providers.
//!
//! Handles:
//! - Base layer migration (curio config new-cluster)
//! - PDP layer configuration (curio config create)

use super::super::step::StepContext;
use super::constants::{DB_SETUP_WAIT_SECS, PDP_LAYER_CONFIG_TEMPLATE};
use crate::commands::start::genesis::constants::PDP_SP_MINER_ID_START;
use crate::constants::CURIO_CONTAINER;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Setup Curio database for a specific PDP SP.
///
/// Steps:
/// 1. Run `curio config new-cluster t0XXXX` for base layer migration
/// 2. Run `curio config create --title pdp-only` with PDP layer config
pub fn setup_curio_database(context: &StepContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!(
        "    {} Setting up database for PDP SP {}...",
        "💾".cyan(),
        sp_index
    );

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = format!("{}-{}-{}", CURIO_CONTAINER, sp_index, run_id);

    // Calculate miner ID for this PDP SP
    let miner_id = format!("t0{}", PDP_SP_MINER_ID_START + (sp_index as u32) - 1);

    // Step 1: Base layer migration
    create_base_cluster(&container_name, &miner_id)?;

    // Step 2: PDP layer configuration
    create_pdp_layer(&container_name, sp_index)?;

    println!(
        "    {} Database setup complete for PDP SP {}",
        "✓".green(),
        sp_index
    );

    Ok(())
}

/// Create base cluster configuration for a miner.
///
/// Runs: `curio config new-cluster <miner_id>`
fn create_base_cluster(container_name: &str, miner_id: &str) -> Result<(), Box<dyn Error>> {
    println!(
        "      {} Creating base cluster for miner {}...",
        "⚙".cyan(),
        miner_id
    );

    let output = Command::new("docker")
        .args([
            "exec",
            container_name,
            "/usr/local/bin/lotus-bins/curio",
            "config",
            "new-cluster",
            miner_id,
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create base cluster for miner {}: {}",
            miner_id,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Wait for DB changes to propagate
    thread::sleep(Duration::from_secs(DB_SETUP_WAIT_SECS));

    println!(
        "      {} Base cluster created for miner {}",
        "✓".green(),
        miner_id
    );

    Ok(())
}

/// Create PDP layer configuration.
///
/// Runs: `curio config create --title pdp-only` with PDP layer config
fn create_pdp_layer(container_name: &str, sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!(
        "      {} Creating PDP layer configuration...",
        "⚙".cyan()
    );

    // Generate PDP layer config with sp_index
    let pdp_config = PDP_LAYER_CONFIG_TEMPLATE.replace("{sp_index}", &sp_index.to_string());

    // Write config via stdin
    let output = Command::new("docker")
        .args([
            "exec",
            "-i",
            container_name,
            "/bin/bash",
            "-c",
            &format!(
                "/usr/local/bin/lotus-bins/curio config create --title pdp-only << 'EOF'\n{}\nEOF",
                pdp_config
            ),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create PDP layer configuration: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("      {} PDP layer configuration created", "✓".green());

    Ok(())
}
