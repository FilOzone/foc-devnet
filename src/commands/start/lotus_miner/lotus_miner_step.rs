//! Lotus-Miner step implementation.
//!
//! This module contains the main Step implementation for starting Lotus-Miner.

use std::error::Error;
use std::path::PathBuf;

use super::container_ops::start_miner_container;
use super::docker_command::build_miner_docker_command;
use super::setup::{find_preseal_files, setup_miner_directories};
use super::verification::perform_post_execution_verification;
use crate::commands::start::step::{Step, StepContext};

/// Step for starting the Lotus-Miner node
pub struct LotusMinerStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl LotusMinerStep {
    /// Create a new LotusMinerStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }
}

impl Step for LotusMinerStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Start Lotus-Miner"
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        setup_miner_directories(&self.volumes_dir)?;
        let preseal_files = find_preseal_files()?;
        let docker_args = build_miner_docker_command(&self.volumes_dir, &preseal_files, context)?;
        start_miner_container(docker_args, context)?;
        Ok(())
    }

    /// Perform post-execution verification for Lotus-Miner startup
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        perform_post_execution_verification(context)
    }
}
