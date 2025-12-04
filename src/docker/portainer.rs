//! Portainer container management for cluster visualization.
//!
//! This module handles starting and stopping Portainer instances for each cluster run.
//! Portainer provides a web UI for managing Docker containers at http://localhost:9009

use super::containers::portainer_container_name;
use super::core::{
    container_exists, container_is_running, docker_command, stop_and_remove_container,
};
use crossterm::style::Stylize;
use std::error::Error;

const PORTAINER_IMAGE: &str = "portainer/portainer-ce:latest";
const PORTAINER_PORT: u16 = 9009;
const PORTAINER_DATA_VOLUME: &str = "portainer_data";

/// Start a Portainer instance for the cluster run
///
/// Portainer will be accessible at http://localhost:9009
///
/// # Arguments
/// * `run_id` - The run ID for this cluster
///
/// # Returns
/// Ok(()) on success, error on failure
pub fn start_portainer(run_id: &str) -> Result<(), Box<dyn Error>> {
    let container_name = portainer_container_name(run_id);

    println!("{}", "Starting Portainer...".blue().bold());
    println!("  Container: {}", container_name);

    // Check if already running
    if container_is_running(&container_name)? {
        println!("  {} Portainer already running", "ℹ".cyan());
        return Ok(());
    }

    // Stop and remove if exists but not running
    if container_exists(&container_name)? {
        println!("  Cleaning up existing container...");
        stop_and_remove_container(&container_name)?;
    }

    // Pull latest Portainer image
    println!("  Pulling Portainer image...");
    docker_command(&["pull", PORTAINER_IMAGE])?;

    // Start Portainer container
    println!("  Starting container...");
    let port_mapping = format!("{}:9000", PORTAINER_PORT);
    let volume_mapping = format!("{}:/data", PORTAINER_DATA_VOLUME);
    docker_command(&[
        "run",
        "-d",
        "--name",
        &container_name,
        "-p",
        &port_mapping,
        "-v",
        "/var/run/docker.sock:/var/run/docker.sock",
        "-v",
        &volume_mapping,
        "--restart",
        "unless-stopped",
        PORTAINER_IMAGE,
    ])?;

    println!("  {} Portainer started", "✓".green());
    println!(
        "  {} Access at: {}",
        "ℹ".cyan(),
        format!("http://localhost:{}", PORTAINER_PORT)
            .yellow()
            .underlined()
    );

    Ok(())
}

/// Stop and remove the Portainer instance for the cluster run
///
/// # Arguments
/// * `run_id` - The run ID for this cluster
///
/// # Returns
/// Ok(()) on success, error on failure
pub fn stop_portainer(run_id: &str) -> Result<(), Box<dyn Error>> {
    let container_name = portainer_container_name(run_id);

    println!("{}", "Stopping Portainer...".blue().bold());

    if !container_exists(&container_name)? {
        println!("  {} Portainer container does not exist", "ℹ".cyan());
        return Ok(());
    }

    stop_and_remove_container(&container_name)?;
    println!("  {} Portainer stopped and removed", "✓".green());

    Ok(())
}
