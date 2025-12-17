//! Pre-execution checks for Curio step.
//!
//! Verifies that Lotus is running and blocks are being generated before
//! attempting to start Curio.

use super::super::step::StepContext;
use crate::constants::{LOTUS_CONTAINER, LOTUS_MINER_CONTAINER};
use crate::docker::{container_is_running, container_exists};
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Verify prerequisites for Curio setup.
///
/// Checks:
/// 1. Lotus container is running
/// 2. Lotus-Miner container is running
/// 3. Chain is progressing (blocks are being generated)
pub fn verify_prerequisites(context: &StepContext, sp_count: usize) -> Result<(), Box<dyn Error>> {
    println!("  {} Verifying Lotus is running and producing blocks...", "🔍".cyan());

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let lotus_container = format!("{}-{}", LOTUS_CONTAINER, run_id);
    let miner_container = format!("{}-{}", LOTUS_MINER_CONTAINER, run_id);

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

    // Check Lotus-Miner container exists and is running
    if !container_exists(&miner_container)? {
        return Err(format!(
            "Lotus-Miner container '{}' does not exist. Run the Lotus-Miner step first.",
            miner_container
        )
        .into());
    }

    if !container_is_running(&miner_container)? {
        return Err(format!(
            "Lotus-Miner container '{}' is not running. Ensure Lotus-Miner step completed successfully.",
            miner_container
        )
        .into());
    }

    // Verify chain is progressing
    verify_chain_progressing(&lotus_container)?;

    println!(
        "  {} Prerequisites verified: Lotus is running and producing blocks",
        "✓".green()
    );
    println!(
        "  {} Will activate {} PDP Service Provider(s)",
        "ℹ".cyan(),
        sp_count
    );

    Ok(())
}

/// Verify that the Filecoin chain is progressing (blocks are being generated).
fn verify_chain_progressing(lotus_container: &str) -> Result<(), Box<dyn Error>> {
    println!("    {} Checking chain is progressing...", "⛓".cyan());

    // Get initial block height
    let height1 = get_chain_head_height(lotus_container)?;
    
    // Wait 6 seconds (should be enough for at least 1 block with 4s block time)
    println!("    {} Waiting 6 seconds to verify block production...", "⏳".cyan());
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

    println!(
        "    {} Chain is progressing (height {} → {})",
        "✓".green(),
        height1,
        height2
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
