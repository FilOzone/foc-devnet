use std::io::ErrorKind;
use std::process::Command;
use tracing::{info, warn};

use crate::paths::{foc_devnet_config, foc_devnet_home};

/// Remove foc-devnet state from the home directory.
///
/// Preserves config.toml by default so `init` can reuse it. Pass `all` to
/// remove config.toml too. The directory itself is always preserved to avoid
/// permission errors when the parent is not user-writable (e.g. a mount point).
pub fn clean(all: bool, remove_images: bool) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = foc_devnet_home();
    let config_path = foc_devnet_config();

    if !home_dir.exists() {
        info!("Nothing to clean ({})", home_dir.display());
        return Ok(());
    }

    info!("Cleaning {}", home_dir.display());
    let mut kept_config = false;

    for entry in std::fs::read_dir(&home_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !all && path == config_path {
            kept_config = true;
            continue;
        }

        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }

    if kept_config {
        info!("Preserved config.toml (use --all to remove it too)");
    }

    info!("Cleaned foc-devnet state");

    if remove_images {
        clean_docker_images()?;
    }

    Ok(())
}

/// Check whether the home directory is ready for init.
///
/// Returns true if the directory contains no meaningful state. Ignores
/// config.toml (preserved by clean for reuse) and the state/ and run/
/// directories which are created as side effects of the logging and poison
/// infrastructure before any command runs.
pub fn is_clean_for_init() -> Result<bool, Box<dyn std::error::Error>> {
    let home_dir = foc_devnet_home();
    if !home_dir.exists() {
        return Ok(true);
    }
    let config_path = foc_devnet_config();
    let state_dir = crate::paths::foc_devnet_state();
    let runs_dir = crate::paths::foc_devnet_runs();
    for entry in std::fs::read_dir(&home_dir)? {
        let path = entry?.path();
        if path == config_path || path == state_dir || path == runs_dir {
            continue;
        }
        return Ok(false);
    }
    Ok(true)
}

fn docker_not_found_error() -> Box<dyn std::error::Error> {
    "Docker CLI not found. Install Docker and ensure the 'docker' command is on PATH."
        .to_string()
        .into()
}

fn clean_docker_images() -> Result<(), Box<dyn std::error::Error>> {
    info!("Removing foc-* Docker images");
    let output = Command::new("docker")
        .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
        .output()
        .map_err(|err| match err.kind() {
            ErrorKind::NotFound => docker_not_found_error(),
            _ => err.into(),
        })?;

    if !output.status.success() {
        warn!("Could not list Docker images (Docker may not be running)");
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut removed_count = 0;

    for line in stdout.lines() {
        if line.starts_with("foc-") {
            let remove_output = Command::new("docker")
                .args(["rmi", line])
                .output()
                .map_err(|err| match err.kind() {
                    ErrorKind::NotFound => docker_not_found_error(),
                    _ => err.into(),
                })?;

            if remove_output.status.success() {
                removed_count += 1;
            }
        }
    }

    if removed_count > 0 {
        info!("Removed {} Docker image(s)", removed_count);
    } else {
        info!("No foc-* Docker images found");
    }

    Ok(())
}
