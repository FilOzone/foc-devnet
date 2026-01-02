//! Main entry point for foc-localnet.
//!
//! This module provides the main application entry point with command routing.

use clap::Parser;
use foc_localnet::cli::{Cli, Commands};
use foc_localnet::logger::init_logging;
use foc_localnet::poison;
use foc_localnet::run_id::generate_run_id;

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
        Commands::Start {
            volumes_dir,
            run_dir,
            parallel,
            notest,
        } => {
            main_app::command_handlers::handle_start(volumes_dir, run_dir, parallel, run_id, notest)
        }
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
            no_docker_build,
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
