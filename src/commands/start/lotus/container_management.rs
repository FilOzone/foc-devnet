//! Container management for Lotus daemon.
//!
//! This module contains functions for managing the Lotus daemon Docker container,
//! including starting, stopping, and checking container status.

use super::super::step::StepContext;
use crate::docker::containers::lotus_container_name;
use crate::docker::{container_exists, container_is_running, stop_and_remove_container};
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

// Timing constants
const CONTAINER_INIT_WAIT_SECS: u64 = 10;

// Log constants
const LOG_TAIL_LINES: &str = "50";

/// Get the Lotus container name from context
fn get_container_name(context: &StepContext) -> Result<String, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    Ok(lotus_container_name(run_id))
}

/// Check and handle any existing Lotus container
pub fn check_existing_container(context: &StepContext) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;

    // Check if any existing lotus container is running
    if container_exists(&container_name)? {
        if container_is_running(&container_name)? {
            println!(
                "    {} Container '{}' is already running",
                "⚠".yellow(),
                container_name
            );
            stop_and_remove_container(&container_name)?;
        } else {
            println!(
                "    {} Container '{}' exists but is not running",
                "⚠".yellow(),
                container_name
            );
            stop_and_remove_container(&container_name)?;
        }
    }
    Ok(())
}

/// Start the Lotus daemon container
pub fn start_container(
    docker_args: Vec<String>,
    context: &StepContext,
) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;

    println!(
        "    Starting Lotus daemon container '{}'...",
        container_name
    );
    let output = Command::new("docker").args(&docker_args).output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to start Lotus container: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    context.set("lotus_container_id", container_id.clone());
    context.set("lotus_container_name", container_name);
    println!(
        "    {} Container started with ID: {}",
        "✓".green(),
        &container_id[..12]
    );

    Ok(())
}

/// Wait for the container to initialize after starting
pub fn wait_for_container_init(context: &StepContext) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;

    // Wait for container to initialize
    println!("    Waiting for Lotus daemon to start...");
    thread::sleep(Duration::from_secs(CONTAINER_INIT_WAIT_SECS));

    // Verify container is running
    if !container_is_running(&container_name)? {
        // Check logs for errors
        let logs_output = Command::new("docker")
            .args(["logs", "--tail", LOG_TAIL_LINES, &container_name])
            .output()?;

        return Err(format!(
            "Container stopped unexpectedly. Logs:\n{}",
            String::from_utf8_lossy(&logs_output.stdout)
        )
        .into());
    }
    println!("    {} Container is running", "✓".green());
    Ok(())
}
