use crate::docker::core::{container_exists, container_is_running, docker_command};
use crate::docker::{delete_all_networks, is_foc_devnet_network, list_foc_devnet_containers};
use crate::run_id::{delete_current_run_id, load_current_run_id};
use std::error::Error;
use tracing::{info, warn};

/// Container names for all services
const CONTAINERS: &[(&str, &str)] = &[
    (crate::constants::CURIO_CONTAINER, "Curio"),
    (crate::constants::LOTUS_MINER_CONTAINER, "Lotus-Miner"),
    (crate::constants::LOTUS_CONTAINER, "Lotus"),
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

/// Get container names for a specific run ID, in reverse of start order so
/// dependents go before their dependencies: Curio SPs, then their databases,
/// then Lotus-Miner, then Lotus. Names for SP indices that were never started
/// are skipped by stop_and_remove_service_container.
fn get_run_containers(run_id: &str) -> Vec<(String, &'static str)> {
    let mut containers: Vec<(String, &'static str)> = (1..=crate::constants::MAX_PDP_SP_COUNT)
        .map(|sp| (crate::docker::curio_container_name(run_id, sp), "Curio"))
        .collect();
    // Per-SP database containers run stock images and are invisible to the
    // image-based force-kill sweep. Remove them explicitly by name.
    for name in crate::docker::db_container_names(run_id) {
        containers.push((name, "Database"));
    }
    containers.push((format!("foc-{}-lotus-miner", run_id), "Lotus-Miner"));
    containers.push((format!("foc-{}-lotus", run_id), "Lotus"));
    containers
}

/// Force kill all foc-devnet containers, identified by exact image name. Avoids
/// touching unrelated containers that happen to share the "foc-" prefix
/// (e.g. foc-observer-*).
fn force_kill_foc_containers() -> Result<(), Box<dyn Error>> {
    info!("Force killing any remaining foc-devnet containers...");

    let containers = list_foc_devnet_containers()?;

    if containers.is_empty() {
        info!("No remaining foc-devnet containers found");
        return Ok(());
    }

    info!("Found {} remaining container(s)", containers.len());

    for c in containers {
        info!("Force removing container {}...", c.name);
        match docker_command(&["rm", "-f", &c.name]) {
            Ok(_) => info!("Removed"),
            Err(e) => warn!("Failed: {}", e),
        }
    }

    info!("Force remove containers complete");
    Ok(())
}

/// Force remove all Docker networks belonging to foc-devnet, identified by exact
/// match against the `foc_{run_id}_{lot-net|lot-m-net|cur-m-net-N}` naming scheme.
/// Listing all networks then filtering in Rust avoids edge cases with docker's
/// own filter syntax and any unrelated networks that share a `foc_` prefix.
fn force_remove_foc_networks() -> Result<(), Box<dyn Error>> {
    info!("Force removing any remaining foc-devnet networks...");

    let output = docker_command(&["network", "ls", "--format", "{{.Name}}"])?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let network_names: Vec<&str> = stdout_str
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && is_foc_devnet_network(line))
        .collect();

    if network_names.is_empty() {
        info!("No remaining foc-devnet networks found");
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
