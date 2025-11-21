mod curio;
mod genesis;
mod lotus;
mod lotus_miner;
mod step;
mod yugabyte;

use curio::CurioStep;
pub use genesis::ensure_genesis_prerequisites;
use lotus::LotusStep;
use lotus_miner::LotusMinerStep;
pub use step::{Step, StepContext, execute_steps};
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

    // Handle reset flag - delete genesis-related files and keys
    if reset {
        println!("{}", "Resetting genesis data...".yellow().bold());
        
        let base_volumes = foc_localnet_docker_volumes();
        
        // Files and directories to delete
        let paths_to_delete = vec![
            base_volumes.join("lotus-keys").join("key-1"),
            base_volumes.join("lotus-keys").join("key-2"),
            base_volumes.join("lotus-keys").join("prefunded-1"),
            base_volumes.join("lotus-keys").join("prefunded-2"),
            base_volumes.join("genesis-sectors"),
            base_volumes.join("genesis").join("foc-localnet.json"),
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
        
        println!("{}", "Reset complete.".green().bold());
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
    // 3. YugabyteDB - database for Curio
    // 4. Curio (second gen miner) - needs both Lotus and YugabyteDB
    let lotus_step = LotusStep::new(volumes_dir.clone(), logs_dir.clone());
    let lotus_miner_step = LotusMinerStep::new(volumes_dir.clone(), logs_dir.clone());
    let yugabyte_step = YugabyteStep::new(volumes_dir.clone(), logs_dir.clone());
    let curio_step = CurioStep::new(volumes_dir.clone(), logs_dir.clone());

    // Execute all steps
    let steps: Vec<&dyn Step> = vec![&lotus_step, &lotus_miner_step, &yugabyte_step, &curio_step];
    execute_steps(steps)?;

    println!("\n{}", "Local cluster started successfully!".green().bold());
    Ok(())
}
