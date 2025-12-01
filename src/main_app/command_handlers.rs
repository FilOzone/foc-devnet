//! Command handlers for CLI subcommands.
//!
//! This module contains the execution logic for different CLI commands.

use std::fs;

use foc_localnet::cli::{BuildCommands, ConfigCommands};
use foc_localnet::commands;
use foc_localnet::commands::build::Project;
use foc_localnet::config::Config;
use foc_localnet::paths::foc_localnet_config;
use foc_localnet::poison;

/// Execute the start command
pub fn handle_start(
    volumes_dir: Option<String>,
    logs_dir: Option<String>,
    regenesis: bool,
    reset: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Start")?;
    commands::start_cluster(volumes_dir, logs_dir, regenesis, reset)
}

/// Execute the stop command
pub fn handle_stop() -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Stop")?;
    commands::stop_cluster()
}

/// Execute the requirements command
pub fn handle_requirements(setup: bool) -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Requirements")?;
    commands::check_requirements(setup)
}

/// Execute the init command
pub fn handle_init(
    curio: Option<String>,
    lotus: Option<String>,
    filecoin_services: Option<String>,
    yugabyte_url: Option<String>,
    force: bool,
    rand: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Init")?;
    commands::init_environment(curio, lotus, filecoin_services, yugabyte_url, force, rand)
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

/// Execute the clean command
pub fn handle_clean(
    artifacts: bool,
    dockerimages: bool,
    binaries: bool,
    lotus: bool,
    curio: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Clean")?;
    commands::clean_environment(artifacts, dockerimages, binaries, lotus, curio, false)
}

/// Execute the status command
pub fn handle_status() -> Result<(), Box<dyn std::error::Error>> {
    // Status is read-only, no poison protection needed
    commands::status()
}

/// Execute the config command
pub fn handle_config(config_command: ConfigCommands) -> Result<(), Box<dyn std::error::Error>> {
    poison::create_poison("Config")?;
    match config_command {
        ConfigCommands::Lotus { source } => commands::config_lotus(source),
        ConfigCommands::Curio { source } => commands::config_curio(source),
    }
}
