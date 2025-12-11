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

/// Find an existing Portainer container (from any run)
///
/// # Returns
/// Some(container_name) if a Portainer container exists, None otherwise
fn find_existing_portainer() -> Result<Option<String>, Box<dyn Error>> {
    // List all running containers with names starting with "foc-" and containing "portainer"
    let output = docker_command(&[
        "ps",
        "--filter",
        "name=^portainer*",
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
/// Portainer will be accessible at http://localhost:9009
/// If any Portainer instance already exists, it will be reused instead of creating a new one.
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

    // Check if any Portainer container already exists (from any run)
    let existing_portainer = find_existing_portainer()?;
    if let Some(existing_name) = existing_portainer {
        println!(
            "  {} Reusing existing Portainer container: {}",
            "ℹ".cyan(),
            existing_name
        );
        println!(
            "  {} Access at: {}",
            "ℹ".cyan(),
            format!("http://localhost:{}", PORTAINER_PORT)
                .yellow()
                .underlined()
        );
        return Ok(());
    }

    // Check if our specific container is already running
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
