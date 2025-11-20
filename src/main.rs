use clap::Parser;
use crossterm::style::Stylize;
use foc_localnet::app;
use foc_localnet::cli::{BuildCommands, Cli, Commands};
use foc_localnet::commands;
use foc_localnet::commands::build::Project;
use foc_localnet::config::Config;
use foc_localnet::paths::foc_localnet_config;
use foc_localnet::poison;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::init_tracing();

    let cli = Cli::parse();

    // Check for poison file and attempt recovery
    poison::check_and_recover_poison()?;

    // Execute the command with poison file protection
    let result = match cli.command {
        Commands::Start {
            volumes_dir,
            logs_dir,
        } => {
            poison::create_poison("Start")?;
            commands::start_cluster(volumes_dir, logs_dir)
        }
        Commands::Stop => {
            poison::create_poison("Stop")?;
            commands::stop_cluster()
        }
        Commands::Requirements { setup } => {
            poison::create_poison("Requirements")?;
            commands::check_requirements(setup)
        }
        Commands::Init {
            curio,
            lotus,
            yugabyte_url,
            force,
        } => {
            poison::create_poison("Init")?;
            commands::init_environment(curio, lotus, yugabyte_url, force)
        }
        Commands::Build { build_command } => {
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
        Commands::Clean {
            artifacts,
            dockerimages,
            binaries,
            lotus,
            curio,
        } => {
            poison::create_poison("Clean")?;
            commands::clean_environment(artifacts, dockerimages, binaries, lotus, curio, false)
        }
        Commands::Status => {
            // Status is read-only, no poison protection needed
            commands::status()
        }
    };

    // Handle the result
    match result {
        Ok(_) => {
            // Remove poison file on successful completion
            poison::remove_poison()?;
            Ok(())
        }
        Err(e) => {
            // Leave poison file in place on error
            eprintln!(
                "{}",
                "Command failed, poison file left in place for safety".red()
            );
            Err(e)
        }
    }
}
