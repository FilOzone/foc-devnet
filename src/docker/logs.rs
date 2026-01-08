//! Docker log collection and cleanup utilities.
//!
//! This module provides functions to persist logs from all containers
//! whose images start with the prefix "foc" and to remove any dead
//! containers after a start attempt, regardless of success or failure.
//!
//! The log files are stored under the run-specific directory:
//! ~/.foc-localnet/run/<run_id>/logs/<container_name>.<image_name>.docker.log

use crate::docker::core::{docker_command, get_container_logs};
use crate::paths::foc_localnet_run_dir;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

/// Information about a Docker container for logging and cleanup.
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub status: String,
}

/// List all containers (running or stopped) whose image name starts with the given prefix.
pub fn list_containers_by_image_prefix(prefix: &str) -> Result<Vec<ContainerInfo>, Box<dyn Error>> {
    let output = docker_command(&["ps", "-a", "--format", "{{.Names}}|{{.Image}}|{{.Status}}"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut result = Vec::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            let name = parts[0].trim().to_string();
            let image = parts[1].trim().to_string();
            let status = parts[2].trim().to_string();
            if image.starts_with(prefix) {
                result.push(ContainerInfo {
                    name,
                    image,
                    status,
                });
            }
        }
    }
    Ok(result)
}

/// Persist logs for all containers whose image starts with "foc" under the run logs directory.
pub fn persist_foc_container_logs(run_id: &str) -> Result<(), Box<dyn Error>> {
    let containers = list_containers_by_image_prefix("foc")?;
    let logs_dir = foc_localnet_run_dir(run_id).join("logs");
    fs::create_dir_all(&logs_dir)?;

    info!(
        "Persisting logs for {} foc* containers to {}",
        containers.len(),
        logs_dir.display()
    );

    for c in containers {
        let safe_image = c.image.replace(':', "_");
        let file_path = logs_dir.join(format!("{}.{}.docker.log", c.name, safe_image));
        let content = match get_container_logs(&c.name) {
            Ok(logs) => {
                info!("✓ Captured logs for container '{}'", c.name);
                logs
            }
            Err(e) => {
                warn!("Failed to get logs for container '{}': {}", c.name, e);
                format!("Failed to get logs for container '{}': {}\n", c.name, e)
            }
        };
        fs::write(&file_path, content)?;
    }
    info!("✓ All container logs persisted");
    Ok(())
}

/// Remove all containers whose image starts with "foc" and are not running.
pub fn remove_dead_foc_containers() -> Result<(), Box<dyn Error>> {
    let containers = list_containers_by_image_prefix("foc")?;
    let mut removed_count = 0;

    for c in containers {
        // Heuristic: remove if status contains "Exited" or "Dead" or "Created"
        let status_lower = c.status.to_lowercase();
        let is_dead = status_lower.contains("exited")
            || status_lower.contains("dead")
            || status_lower.contains("created")
            || status_lower.contains("removing")
            || status_lower.contains("paused");
        if is_dead {
            // Best-effort remove; ignore errors so cleanup continues
            match docker_command(&["rm", &c.name]) {
                Ok(_) => {
                    info!(
                        "✓ Removed dead container: {} (status: {})",
                        c.name, c.status
                    );
                    removed_count += 1;
                }
                Err(e) => {
                    warn!("Failed to remove container '{}': {}", c.name, e);
                }
            }
        }
    }
    info!("✓ Removed {} dead foc* containers", removed_count);
    Ok(())
}

/// Write the output of `foc-localnet status` to the run's post-start status log file.
pub fn write_post_start_status_log(run_id: &str) -> Result<PathBuf, Box<dyn Error>> {
    let run_dir = foc_localnet_run_dir(run_id);
    fs::create_dir_all(&run_dir)?;
    let status_file = run_dir.join("post_start_status.log");

    info!("Writing post-start status to: {}", status_file.display());

    let exe = std::env::current_exe()?;
    let output = std::process::Command::new(exe).arg("status").output()?;

    let mut content = String::new();
    content.push_str(&String::from_utf8_lossy(&output.stdout));
    if !output.status.success() {
        content.push_str("\n[status command failed]\n");
        content.push_str(&String::from_utf8_lossy(&output.stderr));
    }

    fs::write(&status_file, content)?;
    info!("✓ Post-start status logged");
    Ok(status_file)
}
