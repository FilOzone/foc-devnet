//! Container operations for Lotus-Miner.
//!
//! This module provides utilities for starting and managing Lotus-Miner containers.

use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

use super::constants::CONTAINER_ID_DISPLAY_LENGTH;
use crate::commands::start::step::StepContext;
use crate::docker::containers::lotus_miner_container_name;
use crate::docker::network::{connect_container_to_network, filecoin_network_name};

/// Get the Lotus-Miner container name from context
pub fn get_container_name(context: &StepContext) -> Result<String, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    Ok(lotus_miner_container_name(run_id))
}

/// Start the Lotus-Miner container
pub fn start_miner_container(
    docker_args: Vec<String>,
    context: &mut StepContext,
) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let filecoin_network = filecoin_network_name(run_id);

    println!("    Starting Lotus-Miner container '{}'...", container_name);
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
    context.set("lotus_miner_container_name", container_name.clone());
    println!(
        "    {} Container started with ID: {}",
        "✓".green(),
        &container_id[..CONTAINER_ID_DISPLAY_LENGTH]
    );

    // Connect to filecoin network for Lotus daemon communication
    println!("    Connecting to filecoin network for Lotus access...");
    connect_container_to_network(&container_name, &filecoin_network)?;
    println!("    {} Connected to filecoin network", "✓".green());

    Ok(())
}
