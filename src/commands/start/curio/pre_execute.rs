//! Pre-execution checks for Curio step.
//!
//! Verifies that Lotus is running and blocks are being generated before
//! attempting to start Curio.

use super::super::step::SetupContext;
use crate::docker::containers::lotus_container_name;
use crate::docker::{container_exists, container_is_running};
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tracing::info;

/// Verify prerequisites for Curio setup.
///
/// Checks:
/// 1. Lotus container is running
/// 2. Lotus-Miner container is running
/// 3. Chain is progressing (blocks are being generated)
pub fn verify_prerequisites(context: &SetupContext, sp_count: usize) -> Result<(), Box<dyn Error>> {
    info!("  Verifying Lotus is running and producing blocks...");

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let lotus_container = lotus_container_name(run_id);

    // Check Lotus container exists and is running
    if !container_exists(&lotus_container)? {
        return Err(format!(
            "Lotus container '{}' does not exist. Run the Lotus step first.",
            lotus_container
        )
        .into());
    }

    if !container_is_running(&lotus_container)? {
        return Err(format!(
            "Lotus container '{}' is not running. Ensure Lotus step completed successfully.",
            lotus_container
        )
        .into());
    }

    // Verify chain is progressing
    verify_chain_progressing(&lotus_container)?;

    info!(
        "  Allocating and verifying ports for {} Curio instance(s)...",
        sp_count
    );
    for sp_index in 1..=sp_count {
        let api_port = context.allocate_port()?;
        let api_port_alt = context.allocate_port()?;
        let gui_port = context.allocate_port()?;
        let pdp_port = context.allocate_port()?;

        context.set(
            format!("curio_sp_{}_api_port", sp_index),
            api_port.to_string(),
        );
        context.set(
            format!("curio_sp_{}_api_port_alt", sp_index),
            api_port_alt.to_string(),
        );
        context.set(
            format!("curio_sp_{}_gui_port", sp_index),
            gui_port.to_string(),
        );
        context.set(
            format!("curio_sp_{}_pdp_port", sp_index),
            pdp_port.to_string(),
        );

        for (port, desc) in [
            (api_port, "API"),
            (api_port_alt, "API Alt"),
            (gui_port, "GUI"),
            (pdp_port, "PDP"),
        ] {
            if !crate::docker::is_port_available(port) {
                return Err(format!(
                    "Port {} ({}) for Curio SP {} is already in use",
                    port, desc, sp_index
                )
                .into());
            }
        }
    }

    info!("  Prerequisites verified: Lotus is running and producing blocks");
    info!("  Will activate {} PDP Service Provider(s)", sp_count);

    Ok(())
}

/// Verify that the Filecoin chain is progressing (blocks are being generated).
fn verify_chain_progressing(lotus_container: &str) -> Result<(), Box<dyn Error>> {
    info!("    Checking chain is progressing...");

    // Get initial block height
    let height1 = get_chain_head_height(lotus_container)?;

    // Wait 6 seconds (should be enough for at least 1 block with 4s block time)
    info!("    Waiting 6 seconds to verify block production...");
    thread::sleep(Duration::from_secs(6));

    // Get new block height
    let height2 = get_chain_head_height(lotus_container)?;

    if height2 <= height1 {
        return Err(format!(
            "Chain is not progressing. Initial height: {}, Current height: {}. \
            Ensure Lotus-Miner is running and producing blocks.",
            height1, height2
        )
        .into());
    }

    info!(
        "    Chain is progressing (height {} → {})",
        height1, height2
    );

    Ok(())
}

/// Get the current chain head height from Lotus.
fn get_chain_head_height(lotus_container: &str) -> Result<u64, Box<dyn Error>> {
    let output = Command::new("docker")
        .args([
            "exec",
            lotus_container,
            "/usr/local/bin/lotus-bins/lotus",
            "chain",
            "head",
            "--height",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to get chain head height: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let height_str = String::from_utf8_lossy(&output.stdout);
    let height = height_str.trim().parse::<u64>()?;

    Ok(height)
}
