//! Foundry project setup for MockUSDFC deployment.
//!
//! This module handles the setup and preparation of the Foundry project
//! for deploying the MockUSDFC contract.

use crate::commands::start::step::SetupContext;
use crate::docker::command_logger::run_and_log_command;
use crate::embedded_assets;
use crate::paths::foc_localnet_run_dir;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tracing::info;

/// Get or create the MockUSDFC project directory from embedded assets
///
/// Extracts the embedded MockUSDFC Foundry project to a temporary directory
/// and returns the path to that directory.
pub fn get_mockusdfc_project_dir(run_id: &str) -> Result<PathBuf, Box<dyn Error>> {
    let run_dir = foc_localnet_run_dir(run_id);
    let extract_target = run_dir.join("mockusdfc-extract");

    // Always clean and re-extract to ensure we have the latest embedded version
    if extract_target.exists() {
        fs::remove_dir_all(&extract_target)?;
    }

    // Extract embedded MockUSDFC project (this creates contracts/MockUSDFC/ subdirectory)
    embedded_assets::extract_mockusdfc_project(&extract_target)?;

    // The actual project is in the contracts/MockUSDFC subdirectory
    let mockusdfc_dir = extract_target.join("contracts").join("MockUSDFC");

    if !mockusdfc_dir.exists() {
        return Err(format!(
            "MockUSDFC directory not found after extraction at: {}",
            mockusdfc_dir.display()
        )
        .into());
    }

    Ok(mockusdfc_dir)
}

/// Setup the Foundry project (install dependencies if needed)
pub fn setup_foundry_project(context: &SetupContext, contract_dir: &PathBuf, run_id: &str) -> Result<(), Box<dyn Error>> {
    let openzeppelin_path = contract_dir.join("lib/openzeppelin-contracts");

    if !openzeppelin_path.exists() {
        info!("Installing OpenZeppelin contracts...");

        // First, initialize git repo if it doesn't exist
        let git_dir = contract_dir.join(".git");
        if !git_dir.exists() {
            info!("Initializing git repository...");
            let key = format!("usdfc_setup_git_init_{}", run_id);
            let output = run_and_log_command(
                "docker",
                &[
                    "run",
                    "--rm",
                    "-v",
                    &format!("{}:/workspace", contract_dir.display()),
                    "foc-builder",
                    "bash",
                    "-c",
                    "cd /workspace && git init && git config user.email 'foc@localnet' && git config user.name 'FOC Localnet'",
                ],
                context,
                &key,
            )?;

            if !output.status.success() {
                return Err(format!(
                    "Failed to initialize git repository: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }

        // Install dependencies
        let key = format!("usdfc_setup_install_deps_{}", run_id);
        let output = run_and_log_command(
            "docker",
            &[
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
            ],
            context,
            &key,
        )?;

        if !output.status.success() {
            return Err(format!(
                "Failed to install dependencies: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        info!("Dependencies installed");
    }

    // Build contracts
    info!("Building MockUSDFC contract...");
    let key = format!("usdfc_setup_build_{}", run_id);
    let output = run_and_log_command(
        "docker",
        &[
            "run",
            "--rm",
            "-v",
            &format!("{}:/workspace", contract_dir.display()),
            "foc-builder",
            "bash",
            "-c",
            "cd /workspace && forge build",
        ],
        context,
        &key,
    )?;

    if !output.status.success() {
        return Err(format!(
            "Failed to build contracts: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    info!("Contracts built");
    Ok(())
}
