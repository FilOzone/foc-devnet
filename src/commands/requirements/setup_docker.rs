//! Docker setup utilities.
//!
//! This module contains functions for installing Docker on various platforms.

use std::process::Command;
use tracing::{error, info, warn};

pub mod linux;
pub mod macos;

/// Attempt to install Docker
pub fn install_docker() -> Result<(), Box<dyn std::error::Error>> {
    // On macOS, use brew to install Docker Desktop
    if cfg!(target_os = "macos") {
        info!("Installing Docker Desktop via Homebrew...");
        let status = Command::new("brew")
            .args(["install", "--cask", "docker"])
            .status()?;
        if status.success() {
            info!("Docker Desktop installed successfully.");
            warn!("Please start Docker Desktop manually if it's not already running.");
            return Ok(());
        } else {
            error!("Failed to install Docker via Homebrew.");
        }
    } else if cfg!(target_os = "linux") {
        // Try to detect the Linux distribution
        if linux::is_ubuntu_or_debian()? {
            info!("Installing Docker CE on Ubuntu/Debian...");
            linux::install_docker_ubuntu()?;
            return Ok(());
        } else {
            error!("Automatic Docker installation is not supported on this Linux distribution.");
            info!("Please install Docker manually for your platform.");
        }
    } else {
        error!("Automatic Docker installation is only supported on macOS and Ubuntu/Debian.");
        info!("Please install Docker manually for your platform.");
    }
    Err("Failed to install Docker".into())
}
