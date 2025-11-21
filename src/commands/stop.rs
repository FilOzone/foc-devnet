use crossterm::style::Stylize;
use std::process::Command;

const CONTAINER_NAME: &str = "foc-yugabyte";

/// Execute the stop command.
///
/// This function handles stopping the local Filecoin cluster.
/// It performs the reverse operations of the start command:
/// - Stops running containers
/// - Verifies containers are stopped
/// - Removes containers to ensure clean state
pub fn stop_cluster() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Stopping local cluster...".green().bold());
    println!();

    // Stop YugabyteDB
    stop_yugabyte()?;

    println!("\n{}", "Local cluster stopped successfully!".green().bold());
    Ok(())
}

/// Stop and remove the YugabyteDB container
fn stop_yugabyte() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Stopping YugabyteDB...".blue().bold());

    // Check if container exists
    let exists = container_exists(CONTAINER_NAME)?;
    if !exists {
        println!(
            "  {} Container '{}' does not exist",
            "ℹ".cyan(),
            CONTAINER_NAME
        );
        return Ok(());
    }

    // Check if container is running
    let is_running = container_is_running(CONTAINER_NAME)?;
    
    if is_running {
        println!("  Stopping container '{}'...", CONTAINER_NAME);
        let output = Command::new("docker")
            .args(["stop", CONTAINER_NAME])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to stop container '{}': {}",
                CONTAINER_NAME,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        println!("  {} Container stopped", "✓".green());

        // Verify container is stopped
        if container_is_running(CONTAINER_NAME)? {
            return Err(format!("Container '{}' is still running after stop command", CONTAINER_NAME).into());
        }
    } else {
        println!(
            "  {} Container '{}' is not running",
            "ℹ".cyan(),
            CONTAINER_NAME
        );
    }

    // Remove the container
    println!("  Removing container '{}'...", CONTAINER_NAME);
    let output = Command::new("docker")
        .args(["rm", CONTAINER_NAME])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to remove container '{}': {}",
            CONTAINER_NAME,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    println!("  {} Container removed", "✓".green());

    // Verify container is removed
    if container_exists(CONTAINER_NAME)? {
        return Err(format!("Container '{}' still exists after removal", CONTAINER_NAME).into());
    }

    println!("{}", "YugabyteDB stopped successfully".green());
    Ok(())
}

/// Check if a container with the given name exists
fn container_exists(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["ps", "-a", "--filter", &format!("name=^{}$", name), "--format", "{{.Names}}"])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(name))
}

/// Check if a container is running
fn container_is_running(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["ps", "--filter", &format!("name=^{}$", name), "--format", "{{.Names}}"])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(name))
}
