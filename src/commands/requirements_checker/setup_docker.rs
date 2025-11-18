//! Docker setup utilities.
//!
//! This module contains functions for installing Docker on various platforms.

use std::process::Command;
use crossterm::style::Stylize;

/// Attempt to install Docker
pub fn install_docker() -> Result<(), Box<dyn std::error::Error>> {
    // On macOS, use brew to install Docker Desktop
    if cfg!(target_os = "macos") {
        println!("{}", "📦 Installing Docker Desktop via Homebrew...".blue());
        let status = Command::new("brew")
            .args(&["install", "--cask", "docker"])
            .status()?;
        if status.success() {
            println!("{}", "✅ Docker Desktop installed successfully.".green());
            println!("{}", "🚀 Please start Docker Desktop manually if it's not already running.".yellow());
            return Ok(());
        } else {
            eprintln!("{}", "❌ Failed to install Docker via Homebrew.".red());
        }
    } else {
        eprintln!("{}", "❌ Automatic Docker installation is only supported on macOS via Homebrew.".red());
        eprintln!("{}", "Please install Docker manually for your platform.".cyan());
    }
    Err("Failed to install Docker".into())
}

/// Attempt to install Homebrew
pub fn install_homebrew() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🍺 Installing Homebrew...".blue());
    let status = Command::new("/bin/bash")
        .arg("-c")
        .arg("curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh | bash")
        .status()?;
    if status.success() {
        println!("{}", "✅ Homebrew installed successfully.".green());
        println!("{}", "🔄 You may need to restart your terminal for Homebrew to be available in PATH.".yellow());
        Ok(())
    } else {
        eprintln!("{}", "❌ Failed to install Homebrew.".red());
        eprintln!("{}", "Please install Homebrew manually from https://brew.sh/".cyan());
        Err("Failed to install Homebrew".into())
    }
}