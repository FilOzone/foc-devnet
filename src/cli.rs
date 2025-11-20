use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    Start {
        /// Directory where docker volumes to be loaded will be stored
        #[arg(long)]
        volumes_dir: Option<String>,
        /// Directory where logs of running docker instances will be stored
        #[arg(long)]
        logs_dir: Option<String>,
    },
    /// Stop the local cluster
    Stop,
    /// Check system requirements
    Requirements {
        /// Automatically install and configure missing dependencies
        #[arg(long)]
        setup: bool,
    },
    /// Initialize foc-localnet by building and caching Docker images
    Init,
    /// Build Filecoin projects in a container
    Build {
        #[command(subcommand)]
        build_command: BuildCommands,
    },
    /// Clean various parts of the foc-localnet environment
    Clean {
        /// Clean downloaded artifacts only
        #[arg(long)]
        artifacts: bool,
        /// Clean Docker images only
        #[arg(long)]
        dockerimages: bool,
        /// Clean built binaries only
        #[arg(long)]
        binaries: bool,
        /// Run 'make clean' in Lotus repository only
        #[arg(long)]
        lotus: bool,
        /// Run 'make clean' in Curio repository only
        #[arg(long)]
        curio: bool,
    },
}

/// Build subcommands
#[derive(Subcommand)]
pub enum BuildCommands {
    /// Build Lotus (lotus and lotus-miner)
    Lotus {
        /// Path to the Lotus source directory (optional, will clone if not provided)
        path: Option<PathBuf>,
        /// Output directory for built binaries
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
    /// Build Curio
    Curio {
        /// Path to the Curio source directory (optional, will clone if not provided)
        path: Option<PathBuf>,
        /// Output directory for built binaries
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
}
