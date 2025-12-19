//! Container management for Lotus daemon.
//!
//! This module contains functions for managing the Lotus daemon Docker container,
//! including starting, stopping, and checking container status.

use super::super::step::SetupContext;
use crate::docker::command_logger::run_and_log_command_strings;
use crate::docker::containers::lotus_container_name;
use crate::docker::{container_exists, container_is_running, stop_and_remove_container};
use std::error::Error;
use std::thread;
use std::time::Duration;
use tracing::info;

// Timing constants
const CONTAINER_INIT_WAIT_SECS: u64 = 10;

// Log constants
const LOG_TAIL_LINES: &str = "50";

/// Get the Lotus container name from context
fn get_container_name(context: &SetupContext) -> Result<String, Box<dyn Error>> {
    let run_id = context.run_id();
    Ok(lotus_container_name(run_id))
}

/// Check and handle any existing Lotus container
pub fn check_existing_container(context: &SetupContext) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;

    // Check if any existing lotus container is running
    if container_exists(&container_name)? {
        if container_is_running(&container_name)? {
            info!("Container '{}' is already running", container_name);
            stop_and_remove_container(&container_name)?;
        } else {
            info!("Container '{}' exists but is not running", container_name);
            stop_and_remove_container(&container_name)?;
        }
    }
    Ok(())
}

/// Start the Lotus daemon container
pub fn start_container(
    docker_args: Vec<String>,
    context: &SetupContext,
) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;

    info!("Starting Lotus daemon container '{}'...", container_name);
    let key = format!("lotus_container_start_{}", container_name);
    let output = run_and_log_command_strings("docker", &docker_args, context, &key)?;

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
    info!("Container started with ID: {}", &container_id[..12]);

    Ok(())
}

/// Wait for the container to initialize after starting
pub fn wait_for_container_init(context: &SetupContext) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;

    // Wait for container to initialize
    info!("Waiting for Lotus daemon to start...");
    thread::sleep(Duration::from_secs(CONTAINER_INIT_WAIT_SECS));

    // Verify container is running
    if !container_is_running(&container_name)? {
        // Check logs for errors
        let key = format!("lotus_container_logs_{}", container_name);
        let logs_output = run_and_log_command_strings(
            "docker",
            &[
                "logs".to_string(),
                "--tail".to_string(),
                LOG_TAIL_LINES.to_string(),
                container_name.clone(),
            ],
            context,
            &key,
        )?;

        return Err(format!(
            "Container stopped unexpectedly. Logs:\n{}",
            String::from_utf8_lossy(&logs_output.stdout)
        )
        .into());
    }
    info!("Container is running");
    Ok(())
}
