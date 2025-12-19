use crate::docker::core::{container_exists, container_is_running, docker_command};
use crate::docker::delete_all_networks;
use crate::run_id::{delete_current_run_id, load_current_run_id};
use std::error::Error;
use tracing::{info, warn};

/// Container names for all services
const CONTAINERS: &[(&str, &str)] = &[
    ("foc-curio", "Curio"),
    ("foc-yugabyte", "YugabyteDB"),
    ("foc-lotus-miner", "Lotus-Miner"),
    ("foc-lotus", "Lotus"),
];

/// Execute the stop command.
///
/// This function handles stopping the local Filecoin cluster.
/// It performs the reverse operations of the start command:
/// - Loads the current run ID
/// - Stops running containers in reverse order
/// - Verifies containers are stopped
/// - Removes containers to ensure clean state
/// - Tears down Docker networks
/// - Note: Portainer is not stopped to allow persistent access across runs
/// - Force-kills any remaining foc-* containers
/// - Deletes the run ID file
pub fn stop_cluster() -> Result<(), Box<dyn Error>> {
    info!("Stopping local cluster...");

    // Load the current run ID
    let run_id = match load_current_run_id() {
        Ok(id) => {
            info!("ℹ Run ID: {}", id);
            id
        }
        Err(_) => {
            warn!("Warning: No active run ID found.");
            info!("Attempting to stop all foc-* containers...");
            // Continue without run ID - will try to clean up any foc-* containers
            String::new()
        }
    };

    // Stop all containers in reverse order (opposite of start order)
    // This ensures dependencies are stopped first
    if !run_id.is_empty() {
        // Use run-specific container names
        let containers = get_run_containers(&run_id);
        for (container_name, service_name) in &containers {
            stop_and_remove_service_container(container_name, service_name)?;
        }
    } else {
        // Fallback: stop legacy containers without run ID
        for (container_name, service_name) in CONTAINERS {
            stop_and_remove_service_container(container_name, service_name)?;
        }
    }

    // Note: Portainer is not stopped to allow persistent access across runs
    info!("ℹ Portainer will remain running for persistent access");

    // Force kill any remaining foc* containers (including stopped ones)
    force_kill_foc_containers()?;

    // Delete Docker networks (now that all containers are stopped/removed)
    if !run_id.is_empty() {
        if let Err(e) = delete_all_networks(&run_id) {
            warn!("Failed to remove run-specific networks: {}", e);
            info!("Will attempt force removal of all foc* networks");
        }
    }

    // Force remove any remaining foc* networks (always called)
    force_remove_foc_networks()?;

    // Delete the run ID file
    delete_current_run_id()?;

    info!("Local cluster stopped successfully!");
    Ok(())
}

/// Stop and remove a single container
fn stop_and_remove_service_container(
    container_name: &str,
    service_name: &str,
) -> Result<(), Box<dyn Error>> {
    info!("Stopping {}...", service_name);

    // Check if container exists
    let exists = container_exists(container_name)?;
    if !exists {
        info!("ℹ Container '{}' does not exist", container_name);
        return Ok(());
    }

    // Check if container is running
    let is_running = container_is_running(container_name)?;

    if is_running {
        info!("Stopping container '{}'...", container_name);
        let output = docker_command(&["stop", container_name])?;

        if !output.status.success() {
            return Err(format!(
                "Failed to stop container '{}': {}",
                container_name,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        info!("✓ Container stopped");

        // Verify container is stopped
        if container_is_running(container_name)? {
            return Err(format!(
                "Container '{}' is still running after stop command",
                container_name
            )
            .into());
        }
    } else {
        info!("ℹ Container '{}' is not running", container_name);
    }

    // Remove the container
    info!("Removing container '{}'...", container_name);
    let output = docker_command(&["rm", container_name])?;

    if !output.status.success() {
        return Err(format!(
            "Failed to remove container '{}': {}",
            container_name,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    info!("✓ Container removed");

    if container_exists(container_name)? {
        return Err(format!("Container '{}' still exists after removal", container_name).into());
    }

    info!("{} stopped successfully", service_name);
    Ok(())
}

/// Get container names for a specific run ID
fn get_run_containers(run_id: &str) -> Vec<(String, &'static str)> {
    vec![
        (format!("foc-{}-curio", run_id), "Curio"),
        (format!("foc-{}-yugabyte", run_id), "YugabyteDB"),
        (format!("foc-{}-lotus-miner", run_id), "Lotus-Miner"),
        (format!("foc-{}-lotus", run_id), "Lotus"),
    ]
}

/// Force kill all containers whose name starts with "foc-"
fn force_kill_foc_containers() -> Result<(), Box<dyn Error>> {
    info!("Force killing any remaining foc* containers...");

    // Get all containers (running and stopped) whose name starts with "foc"
    let output = docker_command(&["ps", "-aq", "--filter", "name=^foc*"])?;
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let container_ids: Vec<&str> = stdout_str
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if container_ids.is_empty() {
        info!("No remaining foc* containers found");
        return Ok(());
    }

    info!("Found {} remaining container(s)", container_ids.len());

    for container_id in container_ids {
        info!("Force removing container {}...", container_id);
        let result = docker_command(&["rm", "-f", container_id]);
        match result {
            Ok(_) => info!("Removed"),
            Err(e) => warn!("Failed: {}", e),
        }
    }

    info!("Force remove containers complete");
    Ok(())
}

/// Force remove all Docker networks starting with "foc-" or "foc_"
fn force_remove_foc_networks() -> Result<(), Box<dyn Error>> {
    info!("Force removing any remaining foc* networks...");

    // Get all networks starting with foc- or foc_
    let output = docker_command(&[
        "network",
        "ls",
        "--filter",
        "name=^foc*",
        "--format",
        "{{.Name}}",
    ])?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let network_names: Vec<&str> = stdout_str
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if network_names.is_empty() {
        info!("No remaining foc-* networks found");
        return Ok(());
    }

    info!("Found {} remaining network(s)", network_names.len());

    for network_name in network_names {
        info!("Removing network {}...", network_name);
        let result = docker_command(&["network", "rm", network_name]);
        match result {
            Ok(_) => info!("Removed"),
            Err(e) => warn!("Failed: {}", e),
        }
    }

    info!("Force remove networks complete");
    Ok(())
}
