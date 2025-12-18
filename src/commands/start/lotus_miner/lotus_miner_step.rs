//! Lotus-Miner step implementation.
//!
//! This module contains the main Step implementation for starting Lotus-Miner.

use std::error::Error;
use std::path::PathBuf;

use super::container_ops::start_miner_container;
use super::docker_command::build_miner_docker_command;
use super::setup::{find_preseal_files, setup_miner_directories};
use super::verification::perform_post_execution_verification;
use crate::commands::start::step::{SetupContext, Step};

/// Step for starting the Lotus-Miner node
pub struct LotusMinerStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    run_dir: PathBuf,
}

impl LotusMinerStep {
    /// Create a new LotusMinerStep
    pub fn new(volumes_dir: PathBuf, run_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            run_dir,
        }
    }
}

impl Step for LotusMinerStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Start Lotus-Miner"
    }

    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        // Allocate port for Lotus-Miner API
        let miner_api_port = context.allocate_port()?;
        context.set("lotus_miner_api_port", miner_api_port.to_string());

        if !crate::docker::is_port_available(miner_api_port) {
            return Err(format!(
                "Port {} for Lotus-Miner API is already in use",
                miner_api_port
            )
            .into());
        }
        Ok(())
    }

    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;

        setup_miner_directories(&self.volumes_dir)?;
        let preseal_files = find_preseal_files(run_id)?;
        let docker_args = build_miner_docker_command(&self.volumes_dir, &preseal_files, context)?;
        start_miner_container(docker_args, context)?;
        Ok(())
    }

    /// Perform post-execution verification for Lotus-Miner startup
    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        perform_post_execution_verification(context, &self.volumes_dir)
    }
}
