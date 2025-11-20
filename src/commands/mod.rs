//! Command execution module.
//!
//! This module contains the implementations for various CLI commands
//! used to manage the local Filecoin cluster.

pub mod build;
pub mod clean;
pub mod config;
pub mod init;
pub mod requirements;
pub mod start;
pub mod status;
pub mod stop;

// Re-export the main command functions for easy access
pub use build::build_project;
pub use clean::clean_environment;
pub use config::{config_curio, config_lotus};
pub use init::init_environment;
pub use requirements::check_requirements;
pub use start::start_cluster;
pub use status::status;
pub use stop::stop_cluster;
