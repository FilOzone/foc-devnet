//! Requirements checker command.
//!
//! This module checks if all system requirements are met to run the foc-localnet system.

use crossterm::style::Stylize;
use std::process::{Command, Stdio};

pub mod setup_docker;

/// Check system requirements
pub fn check_requirements(setup: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔍 Checking system requirements...".bold());

    // Check platform-specific requirements
    check_homebrew_requirement(setup)?;

    // Check Docker requirement
    check_docker_requirement(setup)?;

    println!("{}", "🎉 All requirements met!".green().bold());
    Ok(())
}

/// Check Homebrew requirement on macOS
fn check_homebrew_requirement(setup: bool) -> Result<(), Box<dyn std::error::Error>> {
    // On macOS, check for Homebrew first
    if cfg!(target_os = "macos") {
        if !is_command_available("brew") {
            if setup {
                println!(
                    "{}",
                    "❌ Homebrew not found. Attempting to install Homebrew...".yellow()
                );
                setup_docker::macos::install_homebrew()?;
                // Check again after installation
                if !is_command_available("brew") {
                    eprintln!("{}", "❌ Homebrew installation may have failed. Please restart your terminal and try again.".red());
                    return Err("Homebrew not available".into());
                }
            } else {
                eprintln!(
                    "{}",
                    "❌ Error: Homebrew is not installed or not available in PATH."
                        .red()
                        .bold()
                );
                eprintln!(
                    "{}",
                    "Homebrew is required for automatic Docker installation on macOS.".cyan()
                );
                eprintln!("{}", "Please install Homebrew from https://brew.sh/".cyan());
                eprintln!(
                    "{}",
                    "Or run with --setup flag to attempt automatic installation.".cyan()
                );
                return Err("Homebrew not available".into());
            }
        } else {
            println!("{}", "✅ Homebrew is available.".green());
        }
    }
    Ok(())
}

/// Check Docker requirement
fn check_docker_requirement(setup: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Check if docker is available
    if !is_command_available("docker") {
        if setup {
            println!(
                "{}",
                "❌ Docker not found. Attempting to install Docker...".yellow()
            );
            setup_docker::install_docker()?;
        } else {
            eprintln!(
                "{}",
                "❌ Error: Docker is not installed or not available in PATH."
                    .red()
                    .bold()
            );
            eprintln!(
                "{}",
                "Please install Docker Desktop from https://www.docker.com/products/docker-desktop"
                    .cyan()
            );
            eprintln!(
                "{}",
                "Or run with --setup flag to attempt automatic installation.".cyan()
            );
            return Err("Docker not available".into());
        }
    } else {
        println!("{}", "✅ Docker is available.".green());
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
