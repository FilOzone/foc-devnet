use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLI structure for foc-devnet
#[derive(Parser)]
#[command(name = "foc-devnet")]
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
        /// Run steps in parallel where possible (experimental)
        #[arg(long)]
        parallel: bool,
    },
    /// Stop the local cluster
    Stop,
    /// Remove foc-devnet state. Preserves config.toml unless --all is passed.
    Clean {
        /// Also remove config.toml
        #[arg(long)]
        all: bool,
        /// Also remove cached foc-* Docker images
        #[arg(long)]
        images: bool,
    },
    /// Initialize foc-devnet by building and caching Docker images
    Init {
        /// Curio source location.
        /// Latest tag: 'latesttag' (newest), 'latesttag:<selector>' (e.g. 'latesttag:pdp/v*'),
        /// 'latesttag:<url>:<selector>' (custom repo). Resolved once at init.
        /// Explicit: 'gittag:<tag>', 'gittag:<url>:<tag>', 'gitcommit:<sha>',
        /// 'gitcommit:<url>:<sha>', 'gitbranch:<branch>', 'gitbranch:<url>:<branch>',
        /// 'local:/path/to/curio'.
        #[arg(long)]
        curio: Option<String>,
        /// Lotus source location.
        /// Latest tag: 'latesttag' (newest), 'latesttag:<selector>' (e.g. 'latesttag:v*'),
        /// 'latesttag:<url>:<selector>' (custom repo). Resolved once at init.
        /// Explicit: 'gittag:<tag>', 'gittag:<url>:<tag>', 'gitcommit:<sha>',
        /// 'gitcommit:<url>:<sha>', 'gitbranch:<branch>', 'gitbranch:<url>:<branch>',
        /// 'local:/path/to/lotus'.
        #[arg(long)]
        lotus: Option<String>,
        /// Filecoin Services source location.
        /// Latest tag: 'latesttag' (newest), 'latesttag:<selector>' (e.g. 'latesttag:v*'),
        /// 'latesttag:<url>:<selector>' (custom repo). Resolved once at init.
        /// Explicit: 'gittag:<tag>', 'gittag:<url>:<tag>', 'gitcommit:<sha>',
        /// 'gitcommit:<url>:<sha>', 'gitbranch:<branch>', 'gitbranch:<url>:<branch>',
        /// 'local:/path/to/filecoin-services'.
        #[arg(long)]
        filecoin_services: Option<String>,
        /// Yugabyte download URL
        #[arg(long)]
        yugabyte_url: Option<String>,
        /// Path to local Yugabyte archive file (.tar.gz) to use instead of downloading
        #[arg(long)]
        yugabyte_archive: Option<String>,
        /// Path to local filecoin-proof-params directory to use instead of downloading
        #[arg(long)]
        proof_params_dir: Option<String>,
        /// Use random mnemonic instead of deterministic one
        #[arg(long)]
        rand: bool,
        /// Skip building Docker images (useful when images are already cached)
        #[arg(long)]
        no_docker_build: bool,
    },
    /// Build Filecoin projects in a container
    Build {
        #[command(subcommand)]
        build_command: BuildCommands,
    },
    /// Show status of the foc-devnet system
    Status,
    /// Show version information
    Version {
        /// Force plain output without tracing prefixes, even when stdout is a terminal
        #[arg(long)]
        notty: bool,
    },
}

/// Build subcommands
#[derive(Subcommand)]
pub enum BuildCommands {
    /// Build Lotus (lotus and lotus-miner)
    Lotus {
        /// Path to the Lotus source directory (optional, will clone if not provided)
        path: Option<PathBuf>,
    },
    /// Build Curio
    Curio {
        /// Path to the Curio source directory (optional, will clone if not provided)
        path: Option<PathBuf>,
    },
}

/// Config subcommands
#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Configure Lotus source location
    Lotus {
        /// Lotus source location (e.g., 'latesttag', 'latesttag:v*', 'latesttag:<url>:v*', 'gittag:v1.0.0', 'gitcommit:abc123', 'gitbranch:main', 'local:/path/to/lotus')
        source: String,
    },
    /// Configure Curio source location
    Curio {
        /// Curio source location (e.g., 'latesttag', 'latesttag:pdp/v*', 'latesttag:<url>:pdp/v*', 'gittag:v1.0.0', 'gitcommit:abc123', 'gitbranch:main', 'local:/path/to/curio')
        source: String,
    },
}
