use clap::Parser;
use foc_localnet::app;
use foc_localnet::cli::{Cli, Commands};
use foc_localnet::commands;
use foc_localnet::poison;

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
            app::initialize_app()?;
            commands::start_cluster(volumes_dir, logs_dir)
        }
        Commands::Stop => {
            poison::create_poison("Stop")?;
            app::initialize_app()?;
            commands::stop_cluster()
        }
        Commands::RequirementsChecker { setup } => {
            poison::create_poison("RequirementsChecker")?;
            commands::check_requirements(setup)
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
            eprintln!("Command failed, poison file left in place for safety");
            Err(e)
        }
    }
}
