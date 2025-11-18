use clap::{Parser, Subcommand};

/// CLI structure for foc-localnet
#[derive(Parser)]
#[command(name = "foc-localnet")]
#[command(about = "CLI for managing local filecoin-onchain-cloud cluster")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands
#[derive(Subcommand)]
pub enum Commands {
    /// Start the local cluster
    Start,
    /// Stop the local cluster
    Stop,
    /// Check system requirements
    RequirementsChecker {
        /// Automatically install and configure missing dependencies
        #[arg(long)]
        setup: bool,
    },
}
