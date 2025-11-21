mod step;
mod yugabyte;

pub use step::{execute_steps, Step, StepContext};
use yugabyte::YugabyteStep;

use crate::paths::foc_localnet_logs;
use crossterm::style::Stylize;
use std::path::PathBuf;

/// Execute the start command.
///
/// This function handles starting the local Filecoin cluster.
pub fn start_cluster(
    volumes_dir: Option<String>,
    logs_dir: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine volumes directory
    let volumes_dir = if let Some(dir) = volumes_dir {
        PathBuf::from(dir)
    } else {
        // Create a temporary directory for volumes
        std::env::temp_dir().join("foc-localnet-volumes")
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

    // Create steps
    let yugabyte_step = YugabyteStep::new(volumes_dir.clone(), logs_dir.clone());

    // Execute all steps
    let steps: Vec<&dyn Step> = vec![&yugabyte_step];
    execute_steps(steps)?;

    println!("\n{}", "Local cluster started successfully!".green().bold());
    Ok(())
}
