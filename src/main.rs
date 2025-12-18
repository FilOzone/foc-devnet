//! Main entry point for foc-localnet.
//!
//! This module provides the main application entry point with command routing.

use clap::Parser;
use crossterm::style::Stylize;
use foc_localnet::cli::{Cli, Commands};
use foc_localnet::poison;

mod main_app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Check for poison file and attempt recovery
    poison::check_and_recover_poison()?;

    // Execute the command with poison file protection
    let result = match cli.command {
        Commands::Start {
            volumes_dir,
            logs_dir,
            parallel,
        } => main_app::command_handlers::handle_start(volumes_dir, logs_dir, parallel),
        Commands::Stop => main_app::command_handlers::handle_stop(),
        Commands::Init {
            curio,
            lotus,
            filecoin_services,
            synapse_sdk,
            yugabyte_url,
            yugabyte_archive,
            proof_params_dir,
            force,
            rand,
        } => main_app::command_handlers::handle_init(
            curio,
            lotus,
            filecoin_services,
            synapse_sdk,
            yugabyte_url,
            yugabyte_archive,
            proof_params_dir,
            force,
            rand,
        ),
        Commands::Build { build_command } => {
            main_app::command_handlers::handle_build(build_command)
        }
        Commands::Status => main_app::command_handlers::handle_status(),
        Commands::Version => main_app::version::handle_version(),
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
