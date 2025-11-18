//! Docker setup utilities.
//!
//! This module contains functions for installing Docker on various platforms.

use crossterm::style::Stylize;
use std::process::Command;

pub mod linux;
pub mod macos;

/// Attempt to install Docker
pub fn install_docker() -> Result<(), Box<dyn std::error::Error>> {
    // On macOS, use brew to install Docker Desktop
    if cfg!(target_os = "macos") {
        println!("{}", "📦 Installing Docker Desktop via Homebrew...".blue());
        let status = Command::new("brew")
            .args(["install", "--cask", "docker"])
            .status()?;
        if status.success() {
            println!("{}", "✅ Docker Desktop installed successfully.".green());
            println!(
                "{}",
                "🚀 Please start Docker Desktop manually if it's not already running.".yellow()
            );
            return Ok(());
        } else {
            eprintln!("{}", "❌ Failed to install Docker via Homebrew.".red());
        }
    } else if cfg!(target_os = "linux") {
        // Try to detect the Linux distribution
        if linux::is_ubuntu_or_debian()? {
            println!("{}", "📦 Installing Docker CE on Ubuntu/Debian...".blue());
            linux::install_docker_ubuntu()?;
            return Ok(());
        } else {
            eprintln!(
                "{}",
                "❌ Automatic Docker installation is not supported on this Linux distribution."
                    .red()
            );
            eprintln!(
                "{}",
                "Please install Docker manually for your platform.".cyan()
            );
        }
    } else {
        eprintln!(
            "{}",
            "❌ Automatic Docker installation is only supported on macOS and Ubuntu/Debian.".red()
        );
        eprintln!(
            "{}",
            "Please install Docker manually for your platform.".cyan()
        );
    }
    Err("Failed to install Docker".into())
}
