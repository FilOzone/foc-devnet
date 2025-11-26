use crossterm::style::Stylize;
use std::process::Command;

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
/// - Stops running containers in reverse order
/// - Verifies containers are stopped
/// - Removes containers to ensure clean state
pub fn stop_cluster() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Stopping local cluster...".green().bold());
    println!();

    // Stop all containers in reverse order (opposite of start order)
    // This ensures dependencies are stopped first
    for (container_name, service_name) in CONTAINERS {
        stop_container(container_name, service_name)?;
    }

    println!("\n{}", "Local cluster stopped successfully!".green().bold());
    Ok(())
}

/// Stop and remove a single container
fn stop_container(
    container_name: &str,
    service_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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
        let output = Command::new("docker")
            .args(["stop", container_name])
            .output()?;

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
    let output = Command::new("docker")
        .args(["rm", container_name])
        .output()?;

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

/// Check if a container with the given name exists
fn container_exists(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name=^{}$", name),
            "--format",
            "{{.Names}}",
        ])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(name))
}

/// Check if a container is running
fn container_is_running(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            &format!("name=^{}$", name),
            "--format",
            "{{.Names}}",
        ])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(name))
}
