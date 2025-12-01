//! Container management for Lotus daemon.
//!
//! This module contains functions for managing the Lotus daemon Docker container,
//! including starting, stopping, and checking container status.

use super::super::step::StepContext;
use crate::docker::{container_exists, container_is_running, stop_and_remove_container};
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CONTAINER_NAME: &str = "foc-lotus";

// Timing constants
const CONTAINER_INIT_WAIT_SECS: u64 = 10;

// Log constants
const LOG_TAIL_LINES: &str = "50";

/// Check and handle any existing Lotus container
pub fn check_existing_container() -> Result<(), Box<dyn Error>> {
    // Check if any existing lotus container is running
    if container_exists(CONTAINER_NAME)? {
        if container_is_running(CONTAINER_NAME)? {
            println!(
                "    {} Container '{}' is already running",
                "⚠".yellow(),
                CONTAINER_NAME
            );
            stop_and_remove_container(CONTAINER_NAME)?;
        } else {
            println!(
                "    {} Container '{}' exists but is not running",
                "⚠".yellow(),
                CONTAINER_NAME
            );
            stop_and_remove_container(CONTAINER_NAME)?;
        }
    }
    Ok(())
}

/// Start the Lotus daemon container
pub fn start_container(
    docker_args: Vec<String>,
    context: &mut StepContext,
) -> Result<(), Box<dyn Error>> {
    println!(
        "    Starting Lotus daemon container '{}'...",
        CONTAINER_NAME
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
    println!(
        "    {} Container started with ID: {}",
        "✓".green(),
        &container_id[..12]
    );

    Ok(())
}

/// Wait for the container to initialize after starting
pub fn wait_for_container_init() -> Result<(), Box<dyn Error>> {
    // Wait for container to initialize
    println!("    Waiting for Lotus daemon to start...");
    thread::sleep(Duration::from_secs(CONTAINER_INIT_WAIT_SECS));

    // Verify container is running
    if !container_is_running(CONTAINER_NAME)? {
        // Check logs for errors
        let logs_output = Command::new("docker")
            .args(["logs", "--tail", LOG_TAIL_LINES, CONTAINER_NAME])
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