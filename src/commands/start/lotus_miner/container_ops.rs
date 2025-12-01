//! Container operations for Lotus-Miner.
//!
//! This module provides utilities for starting and managing Lotus-Miner containers.

use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

use super::constants::{CONTAINER_ID_DISPLAY_LENGTH, CONTAINER_NAME};
use crate::commands::start::step::StepContext;

/// Start the Lotus-Miner container
pub fn start_miner_container(
    docker_args: Vec<String>,
    context: &mut StepContext,
) -> Result<(), Box<dyn Error>> {
    println!("    Starting Lotus-Miner container '{}'...", CONTAINER_NAME);
    let output = Command::new("docker").args(&docker_args).output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to start Lotus-Miner container: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    context.set("lotus_miner_container_id", container_id.clone());
    println!(
        "    {} Container started with ID: {}",
        "✓".green(),
        &container_id[..CONTAINER_ID_DISPLAY_LENGTH]
    );

    Ok(())
}
