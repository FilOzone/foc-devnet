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
        /// Reset genesis data by deleting keys and genesis sectors before starting
        #[arg(long)]
        reset: bool,
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
    Init {
        /// Curio source location (e.g., 'gittag:tag', 'gittag:url:tag', 'gitcommit:commit', 'gitcommit:url:commit', 'gitbranch:branch', 'gitbranch:url:branch', 'local:/path/to/curio')
        #[arg(long)]
        curio: Option<String>,
        /// Lotus source location (e.g., 'gittag:v1.0.0', 'gittag:url:tag', 'gitcommit:abc123', 'gitcommit:url:commit', 'gitbranch:main', 'gitbranch:url:main', 'local:/path/to/lotus')
        #[arg(long)]
        lotus: Option<String>,
        /// Yugabyte download URL
        #[arg(long)]
        yugabyte_url: Option<String>,
        /// Force regeneration of config file even if it exists
        #[arg(long)]
        force: bool,
    },
    /// Build Filecoin projects in a container
    Build {
        #[command(subcommand)]
        build_command: BuildCommands,
    },
    /// Configure foc-localnet settings
    Config {
        #[command(subcommand)]
        config_command: ConfigCommands,
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
    /// Show status of the foc-localnet system
    Status,
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

/// Config subcommands
#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Configure Lotus source location
    Lotus {
        /// Lotus source location (e.g., 'gittag:v1.0.0', 'gitcommit:abc123', 'local:/path/to/lotus')
        source: String,
    },
    /// Configure Curio source location
    Curio {
        /// Curio source location (e.g., 'gittag:v1.0.0', 'gitcommit:abc123', 'local:/path/to/curio')
        source: String,
    },
}
