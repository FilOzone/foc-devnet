//! Storage management for Curio.
//!
//! Handles attaching fast-storage and long-term-storage locations.

use super::super::step::SetupContext;
use super::constants::{
    CURIO_FAST_STORAGE_PATH, CURIO_LONG_TERM_STORAGE_PATH, STORAGE_ATTACH_WAIT_SECS,
};
use crate::docker::command_logger::run_and_log_command;
use std::error::Error;
use std::thread;
use std::time::Duration;
use tracing::info;

/// Attach storage locations for a specific PDP SP.
///
/// Attaches:
/// 1. Fast storage (seal)
/// 2. Long-term storage (store)
pub fn attach_storage_locations(
    context: &SetupContext,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    info!("Attaching storage locations for PDP SP {}...", sp_index);

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = format!("foc-{}-curio-{}", run_id, sp_index);

    // Attach fast storage
    attach_fast_storage(context, &container_name)?;

    // Attach long-term storage
    attach_long_term_storage(context, &container_name)?;

    info!("Storage locations attached for PDP SP {}", sp_index);

    Ok(())
}

/// Attach fast storage for sealing operations.
fn attach_fast_storage(context: &SetupContext, container_name: &str) -> Result<(), Box<dyn Error>> {
    info!("Attaching fast storage...");

    // Use container DNS name for --machine flag so it works in Docker networks
    let machine_addr = format!("{}:12300", container_name);

    let key = format!("curio_storage_attach_fast_{}", container_name);
    let output = run_and_log_command(
        "docker",
        &[
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
        ],
        context,
        &key,
    )?;

    if !output.status.success() {
        return Err(format!(
            "Failed to attach fast storage: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    thread::sleep(Duration::from_secs(STORAGE_ATTACH_WAIT_SECS));

    info!("Fast storage attached");

    Ok(())
}

/// Attach long-term storage for storing sealed sectors.
fn attach_long_term_storage(context: &SetupContext, container_name: &str) -> Result<(), Box<dyn Error>> {
    info!("Attaching long-term storage...");

    // Use container DNS name for --machine flag so it works in Docker networks
    let machine_addr = format!("{}:12300", container_name);

    let key = format!("curio_storage_attach_long_term_{}", container_name);
    let output = run_and_log_command(
        "docker",
        &[
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
        ],
        context,
        &key,
    )?;

    if !output.status.success() {
        return Err(format!(
            "Failed to attach long-term storage: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    thread::sleep(Duration::from_secs(STORAGE_ATTACH_WAIT_SECS));

    info!("Long-term storage attached");

    Ok(())
}
