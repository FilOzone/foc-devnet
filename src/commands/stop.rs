use crate::docker::core::{container_exists, container_is_running, docker_command};
use crate::docker::delete_all_networks;
use crate::run_id::{delete_current_run_id, load_current_run_id};
use crossterm::style::Stylize;
use std::error::Error;

// Container names for all services
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
    println!("{}", "Stopping local cluster...".green().bold());
    println!();

    // Load the current run ID
    let run_id = match load_current_run_id() {
        Ok(id) => {
            println!("  {} Run ID: {}", "ℹ".cyan(), id);
            id
        }
        Err(_) => {
            println!("{}", "Warning: No active run ID found.".yellow());
            println!("  Attempting to stop all foc-* containers...");
            // Continue without run ID - will try to clean up any foc-* containers
            String::new()
        }
    };
    println!();

    // Stop all containers in reverse order (opposite of start order)
    // This ensures dependencies are stopped first
    if !run_id.is_empty() {
        // Use run-specific container names
        let containers = get_run_containers(&run_id);
        for (container_name, service_name) in &containers {
            stop_container(container_name, service_name)?;
        }
    } else {
        // Fallback: stop legacy containers without run ID
        for (container_name, service_name) in CONTAINERS {
            stop_container(container_name, service_name)?;
        }
    }

    // Note: Portainer is not stopped to allow persistent access across runs
    println!(
        "  {} Portainer will remain running for persistent access",
        "ℹ".cyan()
    );

    // Delete Docker networks
    if !run_id.is_empty() {
        delete_all_networks(&run_id)?;
        println!();
    }

    // Force kill any remaining foc* containers
    force_kill_foc_containers()?;

    // Force delete any remaining foc* networks
    force_remove_foc_networks()?;

    // Delete the run ID file
    delete_current_run_id()?;

    println!("\n{}", "Local cluster stopped successfully!".green().bold());
    Ok(())
}

/// Stop and remove a single container
fn stop_container(container_name: &str, service_name: &str) -> Result<(), Box<dyn Error>> {
    println!("{}", format!("Stopping {}...", service_name).blue().bold());

    // Check if container exists
    let exists = container_exists(container_name)?;
    if !exists {
        println!(
            "  {} Container '{}' does not exist",
            "ℹ".cyan(),
            container_name
        );
        return Ok(());
    }

    // Check if container is running
    let is_running = container_is_running(container_name)?;

    if is_running {
        println!("  Stopping container '{}'...", container_name);
        let output = docker_command(&["stop", container_name])?;

        if !output.status.success() {
            return Err(format!(
                "Failed to stop container '{}': {}",
                container_name,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        println!("  {} Container stopped", "✓".green());

        // Verify container is stopped
        if container_is_running(container_name)? {
            return Err(format!(
                "Container '{}' is still running after stop command",
                container_name
            )
            .into());
        }
    } else {
        println!(
            "  {} Container '{}' is not running",
            "ℹ".cyan(),
            container_name
        );
    }

    // Remove the container
    println!("  Removing container '{}'...", container_name);
    let output = docker_command(&["rm", container_name])?;

    if !output.status.success() {
        return Err(format!(
            "Failed to remove container '{}': {}",
            container_name,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    println!("  {} Container removed", "✓".green());

    // Verify container is removed
    if container_exists(container_name)? {
        return Err(format!("Container '{}' still exists after removal", container_name).into());
    }

    println!(
        "{}",
        format!("{} stopped successfully", service_name).green()
    );
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

/// Force kill all containers whose image starts with "foc-"
fn force_kill_foc_containers() -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        "Force killing any remaining foc* containers..."
            .blue()
            .bold()
    );

    // Get all running containers
    let output = docker_command(&["ps", "-q", "--filter", "name=^foc*"])?;
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let container_ids: Vec<&str> = stdout_str
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if container_ids.is_empty() {
        println!("  {} No remaining foc* containers found", "ℹ".cyan());
        return Ok(());
    }

    println!("  Found {} remaining container(s)", container_ids.len());

    for container_id in container_ids {
        println!("  Killing container {}...", container_id);
        let _ = docker_command(&["kill", container_id]); // Ignore errors
        let _ = docker_command(&["rm", container_id]); // Ignore errors
    }

    println!("  {} Force kill complete", "✓".green());
    Ok(())
}

/// Force remove all Docker networks starting with "foc-" or "foc_"
fn force_remove_foc_networks() -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        "Force removing any remaining foc* networks..."
            .blue()
            .bold()
    );

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
        println!("  {} No remaining foc-* networks found", "ℹ".cyan());
        return Ok(());
    }

    println!("  Found {} remaining network(s)", network_names.len());

    for network_name in network_names {
        println!("  Removing network {}...", network_name);
        let result = docker_command(&["network", "rm", network_name]);
        match result {
            Ok(_) => println!("    {} Removed", "✓".green()),
            Err(e) => println!("    {} Failed: {}", "⚠".yellow(), e),
        }
    }

    println!("  {} Force remove networks complete", "✓".green());
    Ok(())
}
