//! Main entry point for foc-devnet.
//!
//! This module provides the main application entry point with command routing.

use clap::Parser;
use foc_devnet::cli::{Cli, Commands};
use foc_devnet::logger::init_logging;
use foc_devnet::poison;
use foc_devnet::run_id::generate_run_id;

mod main_app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Generate a run ID for this execution and initialize logging
    let run_id = generate_run_id();
    init_logging(&run_id)?;

    // Check for poison file and attempt recovery
    poison::check_and_recover_poison()?;

    // Execute the command with poison file protection
    let result = match cli.command {
        Commands::Start { parallel } => {
            main_app::command_handlers::handle_start(parallel, run_id)
        }
        Commands::Stop => main_app::command_handlers::handle_stop(),
        Commands::Init {
            curio,
            lotus,
            filecoin_services,
            yugabyte_url,
            yugabyte_archive,
            proof_params_dir,
            force,
            rand,
            no_docker_build,
        } => main_app::command_handlers::handle_init(
            curio,
            lotus,
            filecoin_services,
            yugabyte_url,
            yugabyte_archive,
            proof_params_dir,
            force,
            rand,
            no_docker_build,
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
        Err(e) => Err(e),
    }
}
