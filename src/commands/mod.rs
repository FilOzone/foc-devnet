//! Command execution module.
//!
//! This module contains the implementations for various CLI commands
//! used to manage the local Filecoin cluster.

pub mod build;
pub mod config;
pub mod init;
pub mod requirements;
pub mod start;
pub mod status;
pub mod stop;

// Re-export the main command functions for easy access
pub use build::build_project;
pub use config::{config_curio, config_lotus};
pub use init::init_environment;
pub use requirements::check_requirements;
pub use status::status;
pub use stop::stop_cluster;

pub fn start_cluster(parallel: bool, run_id: String) -> Result<(), Box<dyn std::error::Error>> {
    start::start_cluster(parallel, run_id)
}
