mod contract_addresses;
mod curio;
mod env_vars;
mod eth_acc_funding;
mod foc_deploy;
mod foc_deployer;
mod genesis;
mod lotus;
mod lotus_miner;
mod lotus_utils;
mod multicall3_deploy;
mod step;
mod usdfc_deploy;
mod yugabyte;

use curio::CurioStep;
use eth_acc_funding::ETHAccFundingStep;
use foc_deploy::FOCDeployStep;
pub use genesis::ensure_genesis_prerequisites;
use lotus::LotusStep;
use lotus_miner::LotusMinerStep;
use multicall3_deploy::MultiCall3DeployStep;
pub use step::{execute_steps, Step, StepContext};
use usdfc_deploy::USDFCDeployStep;
use yugabyte::YugabyteStep;

use crate::docker::core::{container_is_running, remove_container, stop_container};
use crate::docker::{create_all_networks, start_portainer};
use crate::paths::{contract_addresses_file, foc_localnet_docker_volumes, foc_localnet_run_logs};
use crate::run_id::{generate_run_id, save_current_run_id};
use crate::version_info::write_version_file;
use crossterm::style::Stylize;
pub use eth_acc_funding::constants::FEVM_ACCOUNTS_PREFUNDED;
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
    // Generate run ID for this execution
    let run_id = generate_run_id();

    // Save run ID to persistent storage
    save_current_run_id(&run_id)?;

    // Determine volumes directory
    let volumes_dir = if let Some(dir) = volumes_dir {
        PathBuf::from(dir)
    } else {
        // Create a temporary directory for volumes
        foc_localnet_docker_volumes()
    };

    // Determine logs directory - use run-specific directory
    let logs_dir = if let Some(dir) = logs_dir {
        PathBuf::from(dir)
    } else {
        foc_localnet_run_logs(&run_id)
    };

    // Create directories if they don't exist
    std::fs::create_dir_all(&volumes_dir)?;
    std::fs::create_dir_all(&logs_dir)?;

    // Write version information to the run directory
    let version_info = crate::version_info::VersionInfo::from_env();
    write_version_file(&logs_dir, &version_info)?;

    // Handle regenesis flag - delete genesis-related files and keys
    if regenesis {
        println!("{}", "Performing regenesis (full reset)...".yellow().bold());

        // First, stop any running containers to ensure clean state
        println!("  Stopping any running containers...");
        let containers = vec!["foc-lotus-miner", "foc-lotus", "foc-curio", "foc-yugabyte"];
        for container in containers {
            if container_is_running(container)? {
                println!("    Stopping container '{}'...", container);
                stop_container(container)?;
                remove_container(container)?;
            }
        }

        let base_volumes = foc_localnet_docker_volumes();

        // Files and directories to delete
        let paths_to_delete = vec![
            base_volumes.join("lotus-keys"),
            base_volumes.join("genesis-sectors"),
            base_volumes.join("genesis").join("foc-localnet.json"),
            base_volumes.join("lotus-data"),
            base_volumes.join("lotus-miner-data"),
            contract_addresses_file(),
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
    if reset && !regenesis {
        println!(
            "{}",
            "Resetting lotus and lotus-miner to block 0..."
                .yellow()
                .bold()
        );

        // Stop lotus-miner and lotus containers
        let containers = vec!["foc-lotus-miner", "foc-lotus"];
        for container in containers {
            if container_is_running(container)? {
                println!("  Stopping container '{}'...", container);
                stop_container(container)?;
                remove_container(container)?;
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
    println!("{}", format!("Run ID: {}", run_id).cyan().bold());
    println!(
        "{}",
        format!("Volumes directory: {}", volumes_dir.display()).cyan()
    );
    println!(
        "{}",
        format!("Logs directory: {}", logs_dir.display()).cyan()
    );
    println!();

    // Step 0: Create Docker networks for this run
    create_all_networks(&run_id)?;
    println!();

    // Step 0.5: Start Portainer for web UI management
    start_portainer(&run_id)?;
    println!();

    // Ensure genesis prerequisites are ready (one-time setup)
    ensure_genesis_prerequisites()?;
    println!();

    // Create steps in the order they need to be started:
    // 1. Lotus (execution node) - needed by others
    // 2. Lotus-Miner (first gen miner) - builds tipsets
    // 3. ETHAccFunding - create and fund Ethereum accounts for FOC deployment
    // 4. USDFCDeploy - deploy MockUSDFC token for FOC contracts
    // 5. MultiCall3Deploy - deploy Multicall3 contract for batched calls
    // 6. FOCDeploy - deploy FOC service contracts (requires Lotus with FEVM)
    // 7. YugabyteDB - database for Curio
    // 8. Curio (second gen miner) - needs Lotus, FOC contracts, and YugabyteDB
    let lotus_step = LotusStep::new(volumes_dir.clone(), logs_dir.clone());
    let lotus_miner_step = LotusMinerStep::new(volumes_dir.clone(), logs_dir.clone());
    let eth_acc_funding_step = ETHAccFundingStep::new(logs_dir.clone());
    let usdfc_deploy_step = USDFCDeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let multicall3_deploy_step = MultiCall3DeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let foc_deploy_step = FOCDeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let yugabyte_step = YugabyteStep::new(volumes_dir.clone(), logs_dir.clone());
    let curio_step = CurioStep::new(volumes_dir.clone(), logs_dir.clone());

    // Execute all steps
    let steps: Vec<&dyn Step> = vec![
        &lotus_step,
        &lotus_miner_step,
        &eth_acc_funding_step,
        &usdfc_deploy_step,
        &multicall3_deploy_step,
        &foc_deploy_step,
        &yugabyte_step,
        &curio_step,
    ];
    execute_steps(steps, run_id, logs_dir)?;

    println!("\n{}", "Local cluster started successfully!".green().bold());
    Ok(())
}
