use clap::Parser;
use foc_localnet::app;
use foc_localnet::cli::{Cli, Commands};
use foc_localnet::commands;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start => {
            app::initialize_app()?;
            commands::start_cluster()?;
        }
        Commands::Stop => {
            app::initialize_app()?;
            commands::stop_cluster()?;
        }
        Commands::RequirementsChecker { setup } => {
            commands::check_requirements(setup)?;
        }
    }

    Ok(())
}
