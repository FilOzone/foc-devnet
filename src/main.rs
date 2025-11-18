use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use tracing::info;

#[derive(Parser)]
#[command(name = "foc-localnet")]
#[command(about = "CLI for managing local filecoin-onchain-cloud cluster")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the local cluster
    Start,
    /// Stop the local cluster
    Stop,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Check and create application directory
    let app_dir = Path::new("/opt/foc-localnet");
    if !app_dir.exists() {
        info!("Creating application directory: {:?}", app_dir);
        fs::create_dir_all(app_dir)?;
    }

    // Check and install default config
    let config_path = app_dir.join("config.toml");
    if !config_path.exists() {
        info!("Installing default config: {:?}", config_path);
        let default_config = r#"
# Default configuration for foc-localnet
cluster_name = "local-cluster"
nodes = 3
"#;
        fs::write(&config_path, default_config)?;
    }

    match cli.command {
        Commands::Start => {
            info!("Starting local cluster...");
            // TODO: Implement start logic
            println!("Local cluster started.");
        }
        Commands::Stop => {
            info!("Stopping local cluster...");
            // TODO: Implement stop logic
            println!("Local cluster stopped.");
        }
    }

    Ok(())
}
