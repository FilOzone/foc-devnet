//! Command handlers for CLI subcommands.
//!
//! This module contains the execution logic for different CLI commands.

use std::fs;

use foc_localnet::cli::BuildCommands;
use foc_localnet::commands;
use foc_localnet::commands::build::Project;
use foc_localnet::config::Config;
use foc_localnet::paths::foc_localnet_config;
use foc_localnet::poison;

/// Execute the start command
pub fn handle_start(
    volumes_dir: Option<String>,
    run_dir: Option<String>,
    parallel: bool,
    run_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Start")?;
    commands::start_cluster(volumes_dir, run_dir, parallel, run_id)
}

/// Execute the stop command
pub fn handle_stop() -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Stop")?;
    commands::stop_cluster()
}

/// Execute the init command
pub fn handle_init(
    curio: Option<String>,
    lotus: Option<String>,
    filecoin_services: Option<String>,
    synapse_sdk: Option<String>,
    yugabyte_url: Option<String>,
    yugabyte_archive: Option<String>,
    proof_params_dir: Option<String>,
    force: bool,
    rand: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Init")?;
    commands::init_environment(
        curio,
        lotus,
        filecoin_services,
        synapse_sdk,
        yugabyte_url,
        yugabyte_archive,
        proof_params_dir,
        force,
        rand,
    )
}

/// Execute the build command
pub fn handle_build(build_command: BuildCommands) -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Build")?;

    // Load configuration
    let config_path = foc_localnet_config();
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    match build_command {
        BuildCommands::Lotus {
            path: _,
            output_dir: _,
        } => commands::build_project(&Project::Lotus, &config),
        BuildCommands::Curio {
            path: _,
            output_dir: _,
        } => commands::build_project(&Project::Curio, &config),
    }
}

/// Execute the status command
pub fn handle_status() -> Result<(), Box<dyn std::error::Error>> {
    // Status is read-only, no poison protection needed
    commands::status()
}
