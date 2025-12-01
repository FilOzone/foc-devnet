//! Foundry project setup for MockUSDFC deployment.
//!
//! This module handles the setup and preparation of the Foundry project
//! for deploying the MockUSDFC contract.

use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;

/// Setup the Foundry project (install dependencies if needed)
pub fn setup_foundry_project(contract_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    let openzeppelin_path = contract_dir.join("lib/openzeppelin-contracts");

    if !openzeppelin_path.exists() {
        println!("      Installing OpenZeppelin contracts...");

        // First, initialize git repo if it doesn't exist
        let git_dir = contract_dir.join(".git");
        if !git_dir.exists() {
            println!("        Initializing git repository...");
            let output = Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "-v",
                    &format!("{}:/workspace", contract_dir.display()),
                    "foc-builder",
                    "bash",
                    "-c",
                    "cd /workspace && git init && git config user.email 'foc@localnet' && git config user.name 'FOC Localnet'",
                ])
                .output()?;

            if !output.status.success() {
                return Err(format!(
                    "Failed to initialize git repository: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }

        // Install dependencies
        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-v",
                &format!("{}:/workspace", contract_dir.display()),
                "foc-builder",
                "bash",
                "-c",
                "cd /workspace && \
                 forge install OpenZeppelin/openzeppelin-contracts@v5.0.0 && \
                 forge install foundry-rs/forge-std",
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to install dependencies: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        println!("        {} Dependencies installed", "✓".green());
    }

    // Build contracts
    println!("      Building MockUSDFC contract...");
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/workspace", contract_dir.display()),
            "foc-builder",
            "bash",
            "-c",
            "cd /workspace && forge build",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to build contracts: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("        {} Contracts built", "✓".green());
    Ok(())
}