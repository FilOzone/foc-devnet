//! Storage management for Curio.
//!
//! Handles attaching fast-storage and long-term-storage locations.

use super::super::step::StepContext;
use super::constants::{
    CURIO_FAST_STORAGE_PATH, CURIO_LONG_TERM_STORAGE_PATH, STORAGE_ATTACH_WAIT_SECS,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Attach storage locations for a specific PDP SP.
///
/// Attaches:
/// 1. Fast storage (seal)
/// 2. Long-term storage (store)
pub fn attach_storage_locations(
    context: &StepContext,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    println!(
        "    {} Attaching storage locations for PDP SP {}...",
        "💿".cyan(),
        sp_index
    );

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = format!("foc-{}-curio-{}", run_id, sp_index);

    // Attach fast storage
    attach_fast_storage(&container_name)?;

    // Attach long-term storage
    attach_long_term_storage(&container_name)?;

    println!(
        "    {} Storage locations attached for PDP SP {}",
        "✓".green(),
        sp_index
    );

    Ok(())
}

/// Attach fast storage for sealing operations.
fn attach_fast_storage(container_name: &str) -> Result<(), Box<dyn Error>> {
    println!("      {} Attaching fast storage...", "⚙".cyan());

    // Use container DNS name for --machine flag so it works in Docker networks
    let machine_addr = format!("{}:12300", container_name);

    let output = Command::new("docker")
        .args([
            "exec",
            container_name,
            "/usr/local/bin/lotus-bins/curio",
            "cli",
            "--machine",
            &machine_addr,
            "storage",
            "attach",
            "--init",
            "--seal",
            CURIO_FAST_STORAGE_PATH,
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to attach fast storage: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    thread::sleep(Duration::from_secs(STORAGE_ATTACH_WAIT_SECS));

    println!("      {} Fast storage attached", "✓".green());

    Ok(())
}

/// Attach long-term storage for storing sealed sectors.
fn attach_long_term_storage(container_name: &str) -> Result<(), Box<dyn Error>> {
    println!("      {} Attaching long-term storage...", "⚙".cyan());

    // Use container DNS name for --machine flag so it works in Docker networks
    let machine_addr = format!("{}:12300", container_name);

    let output = Command::new("docker")
        .args([
            "exec",
            container_name,
            "/usr/local/bin/lotus-bins/curio",
            "cli",
            "--machine",
            &machine_addr,
            "storage",
            "attach",
            "--init",
            "--store",
            CURIO_LONG_TERM_STORAGE_PATH,
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to attach long-term storage: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    thread::sleep(Duration::from_secs(STORAGE_ATTACH_WAIT_SECS));

    println!("      {} Long-term storage attached", "✓".green());

    Ok(())
}
