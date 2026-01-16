//! Requirements checker command.
//!
//! This module checks if all system requirements are met to run the foc-devnet system.

use std::process::{Command, Stdio};
use tracing::{error, info, warn};

pub mod setup_docker;

/// Check system requirements
pub fn check_requirements(setup: bool) -> Result<(), Box<dyn std::error::Error>> {
    info!("Checking system requirements...");

    // Check platform-specific requirements
    check_homebrew_requirement(setup)?;

    // Check Docker requirement
    check_docker_requirement(setup)?;

    info!("All requirements met!");
    Ok(())
}

/// Check Homebrew requirement on macOS
fn check_homebrew_requirement(setup: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !is_macos() {
        return Ok(());
    }

    if is_command_available("brew") {
        info!("Homebrew is available.");
        return Ok(());
    }

    if setup {
        warn!("Homebrew not found. Attempting to install Homebrew...");
        install_homebrew()?;
        // Check again after installation
        if !is_command_available("brew") {
            error!("Homebrew installation may have failed. Please restart your terminal and try again.");
            return Err("Homebrew not available".into());
        }
    } else {
        error!("Error: Homebrew is not installed or not available in PATH.");
        info!("Homebrew is required for automatic Docker installation on macOS.");
        info!("Please install Homebrew from https://brew.sh/");
        info!("Or run with --setup flag to attempt automatic installation.");
        return Err("Homebrew not available".into());
    }

    Ok(())
}

/// Check if the current platform is macOS.
fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

/// Install Homebrew on macOS.
fn install_homebrew() -> Result<(), Box<dyn std::error::Error>> {
    setup_docker::macos::install_homebrew()
}

/// Check Docker requirement
fn check_docker_requirement(setup: bool) -> Result<(), Box<dyn std::error::Error>> {
    if is_command_available("docker") {
        info!("Docker is available.");
        return Ok(());
    }

    if setup {
        warn!("Docker not found. Attempting to install Docker...");
        install_docker()?;
        // Check again after installation
        if !is_command_available("docker") {
            error!(
                "Docker installation may have failed. Please restart your terminal and try again."
            );
            return Err("Docker not available".into());
        }
    } else {
        error!("Error: Docker is not installed or not available in PATH.");
        info!("Docker is required to run the Filecoin devnet.");
        info!("Please install Docker from https://www.docker.com/");
        info!("Or run with --setup flag to attempt automatic installation.");
        return Err("Docker not available".into());
    }

    Ok(())
}

/// Install Docker based on the platform.
fn install_docker() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "linux") {
        if setup_docker::linux::is_ubuntu_or_debian()? {
            setup_docker::linux::install_docker_ubuntu()?;
        } else {
            eprintln!("❌ Automatic Docker installation is only supported on Ubuntu/Debian Linux.");
            return Err("Unsupported Linux distribution".into());
        }
    } else if cfg!(target_os = "macos") {
        // On macOS, Docker installation is handled by Homebrew
        eprintln!("❌ Please install Docker Desktop manually on macOS.");
        eprintln!("Download from: https://www.docker.com/products/docker-desktop");
        return Err("Manual Docker installation required on macOS".into());
    } else {
        eprintln!("❌ Automatic Docker installation is not supported on this platform.");
        return Err("Unsupported platform".into());
    }

    Ok(())
}

/// Check if a command is available in PATH
fn is_command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
