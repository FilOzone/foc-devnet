//! Portainer container management for cluster visualization.
//!
//! This module handles starting and stopping Portainer instances for each cluster run.
//! Portainer provides a web UI for managing Docker containers at http://localhost:9009

use super::containers::portainer_container_name;
use super::core::{
    container_exists, container_is_running, docker_command, stop_and_remove_container,
};
use std::error::Error;
use tracing::info;

const PORTAINER_IMAGE: &str = "portainer/portainer-ce:latest";
const PORTAINER_DATA_VOLUME: &str = "portainer_data";

/// Find an existing Portainer container (from any run)
///
/// # Returns
/// Some(container_name) if a Portainer container exists, None otherwise
fn find_existing_portainer() -> Result<Option<String>, Box<dyn Error>> {
    // List all running containers with names starting with "foc-" and containing "portainer"
    let output = docker_command(&[
        "ps",
        "--filter",
        "name=^foc-.*-portainer$",
        "--format",
        "{{.Names}}",
    ])?;
    let stdout_str = String::from_utf8_lossy(&output.stdout);

    let containers: Vec<&str> = stdout_str
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if containers.is_empty() {
        Ok(None)
    } else {
        // Return the first existing Portainer container
        Ok(Some(containers[0].to_string()))
    }
}

/// Start a Portainer instance for the cluster run
///
/// Portainer will be accessible at http://localhost:<port>
///
/// # Arguments
/// * `run_id` - The run ID for this cluster
/// * `port` - The port to use for Portainer
///
/// # Returns
/// Ok(()) on success, error on failure
pub fn start_portainer(run_id: &str, port: u16) -> Result<(), Box<dyn Error>> {
    let container_name = portainer_container_name(run_id);

    info!("{}", "Starting Portainer...");
    info!("Container: {}", container_name);

    // Check if any Portainer container already exists (from any run) and remove it
    // to ensure we use the new port and follow the new naming convention
    let existing_portainer = find_existing_portainer()?;
    if let Some(existing_name) = existing_portainer {
        if existing_name != container_name {
            info!("Removing existing Portainer container: {}", existing_name);
            stop_and_remove_container(&existing_name)?;
        }
    }

    // Check if our specific container is already running
    if container_is_running(&container_name)? {
        info!("Portainer already running");
        return Ok(());
    }

    // Stop and remove if exists but not running
    if container_exists(&container_name)? {
        info!("Cleaning up existing container...");
        stop_and_remove_container(&container_name)?;
    }

    // Pull latest Portainer image
    info!("Pulling Portainer image...");
    docker_command(&["pull", PORTAINER_IMAGE])?;

    // Start Portainer container
    info!("Starting container...");
    let port_mapping = format!("{}:9000", port);
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

    info!("ℹ Portainer started");
    info!("Access at: {}", format!("http://localhost:{}", port));

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

    println!("{}", "Stopping Portainer...");

    if !container_exists(&container_name)? {
        println!(" Portainer container does not exist");
        return Ok(());
    }

    stop_and_remove_container(&container_name)?;
    println!(" Portainer stopped and removed");

    Ok(())
}
