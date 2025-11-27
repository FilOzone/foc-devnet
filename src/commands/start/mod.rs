mod curio;
mod foc_deploy;
mod genesis;
mod lotus;
mod lotus_miner;
mod step;
mod yugabyte;

use curio::CurioStep;
use foc_deploy::FOCDeployStep;
pub use genesis::ensure_genesis_prerequisites;
use lotus::LotusStep;
use lotus_miner::LotusMinerStep;
pub use step::{execute_steps, Step, StepContext};
use yugabyte::YugabyteStep;

use crate::paths::{foc_localnet_docker_volumes, foc_localnet_logs};
use crossterm::style::Stylize;
use std::path::PathBuf;

/// Execute the start command.
///
/// This function handles starting the local Filecoin cluster.
pub fn start_cluster(
    volumes_dir: Option<String>,
    logs_dir: Option<String>,
    regenesis: bool,
    reset: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine volumes directory
    let volumes_dir = if let Some(dir) = volumes_dir {
        PathBuf::from(dir)
    } else {
        // Create a temporary directory for volumes
        foc_localnet_docker_volumes()
    };

    // Determine logs directory
    let logs_dir = if let Some(dir) = logs_dir {
        PathBuf::from(dir)
    } else {
        foc_localnet_logs()
    };

    // Create directories if they don't exist
    std::fs::create_dir_all(&volumes_dir)?;
    std::fs::create_dir_all(&logs_dir)?;

    // Handle regenesis flag - delete genesis-related files and keys
    if regenesis {
        println!("{}", "Performing regenesis (full reset)...".yellow().bold());

        // First, stop any running containers to ensure clean state
        println!("  Stopping any running containers...");
        let containers = vec!["foc-lotus-miner", "foc-lotus", "foc-curio", "foc-yugabyte"];
        for container in containers {
            // Check if container is running and stop it
            let is_running = std::process::Command::new("docker")
                .args([
                    "ps",
                    "--filter",
                    &format!("name=^{}$", container),
                    "--format",
                    "{{.Names}}",
                ])
                .output()
                .map(|output| {
                    String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .contains(container)
                })
                .unwrap_or(false);

            if is_running {
                println!("    Stopping container '{}'...", container);
                let _ = std::process::Command::new("docker")
                    .args(["stop", container])
                    .output();
                let _ = std::process::Command::new("docker")
                    .args(["rm", container])
                    .output();
            }
        }

        let base_volumes = foc_localnet_docker_volumes();

        // Files and directories to delete
        let paths_to_delete = vec![
            base_volumes.join("lotus-keys").join("key-1"),
            base_volumes.join("lotus-keys").join("key-2"),
            base_volumes.join("lotus-keys").join("prefunded-1"),
            base_volumes.join("lotus-keys").join("prefunded-2"),
            base_volumes.join("genesis-sectors"),
            base_volumes.join("genesis").join("foc-localnet.json"),
            base_volumes.join("lotus-data"),
            base_volumes.join("lotus-miner-data"),
        ];

        for path in paths_to_delete {
            if path.exists() {
                if path.is_dir() {
                    std::fs::remove_dir_all(&path)?;
                    println!("  {} {}", "Removed directory:".red(), path.display());
                } else {
                    std::fs::remove_file(&path)?;
                    println!("  {} {}", "Removed file:".red(), path.display());
                }
            } else {
                println!("  {} {}", "Skipped (not found):".dim(), path.display());
            }
        }

        println!("{}", "Regenesis complete.".green().bold());
        println!();
    }

    // Handle reset flag - reset lotus and lotus-miner to block 0
    if reset {
        println!(
            "{}",
            "Resetting lotus and lotus-miner to block 0..."
                .yellow()
                .bold()
        );

        // Stop lotus-miner and lotus containers
        let containers = vec!["foc-lotus-miner", "foc-lotus"];
        for container in containers {
            let is_running = std::process::Command::new("docker")
                .args([
                    "ps",
                    "--filter",
                    &format!("name=^{}$", container),
                    "--format",
                    "{{.Names}}",
                ])
                .output()
                .map(|output| {
                    String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .contains(container)
                })
                .unwrap_or(false);

            if is_running {
                println!("  Stopping container '{}'...", container);
                let _ = std::process::Command::new("docker")
                    .args(["stop", container])
                    .output();
                let _ = std::process::Command::new("docker")
                    .args(["rm", container])
                    .output();
            }
        }

        let base_volumes = foc_localnet_docker_volumes();

        // Only delete lotus-data and lotus-miner-data to reset to block 0
        let paths_to_delete = vec![
            base_volumes.join("lotus-data"),
            base_volumes.join("lotus-miner-data"),
        ];

        for path in paths_to_delete {
            if path.exists() {
                if path.is_dir() {
                    std::fs::remove_dir_all(&path)?;
                    println!("  {} {}", "Removed directory:".red(), path.display());
                } else {
                    std::fs::remove_file(&path)?;
                    println!("  {} {}", "Removed file:".red(), path.display());
                }
            } else {
                println!("  {} {}", "Skipped (not found):".dim(), path.display());
            }
        }

        // Delete contract addresses file to allow re-deployment
        let contract_addresses_path = crate::paths::contract_addresses_file();
        if contract_addresses_path.exists() {
            std::fs::remove_file(&contract_addresses_path)?;
            println!(
                "  {} {}",
                "Removed file:".red(),
                contract_addresses_path.display()
            );
        }

        println!("{}", "Reset to block 0 complete.".green().bold());
        println!();
    }

    println!("{}", "Starting local cluster...".green().bold());
    println!(
        "{}",
        format!("Volumes directory: {}", volumes_dir.display()).cyan()
    );
    println!(
        "{}",
        format!("Logs directory: {}", logs_dir.display()).cyan()
    );
    println!();

    // Ensure genesis prerequisites are ready (one-time setup)
    ensure_genesis_prerequisites()?;
    println!();

    // Create steps in the order they need to be started:
    // 1. Lotus (execution node) - needed by others
    // 2. Lotus-Miner (first gen miner) - builds tipsets
    // 3. FOCDeploy - deploy FOC contracts (requires Lotus with FEVM)
    // 4. YugabyteDB - database for Curio
    // 5. Curio (second gen miner) - needs Lotus, FOC contracts, and YugabyteDB
    let lotus_step = LotusStep::new(volumes_dir.clone(), logs_dir.clone());
    let lotus_miner_step = LotusMinerStep::new(volumes_dir.clone(), logs_dir.clone());
    let foc_deploy_step = FOCDeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let yugabyte_step = YugabyteStep::new(volumes_dir.clone(), logs_dir.clone());
    let _curio_step = CurioStep::new(volumes_dir.clone(), logs_dir.clone());

    // Execute all steps
    let steps: Vec<&dyn Step> = vec![
        &lotus_step,
        &lotus_miner_step,
        &foc_deploy_step,
        &yugabyte_step,
    ];
    execute_steps(steps)?;

    println!("\n{}", "Local cluster started successfully!".green().bold());
    Ok(())
}
