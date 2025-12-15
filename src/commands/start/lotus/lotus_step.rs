//! Lotus execution node step implementation.
//!
//! This module contains the LotusStep struct and its implementation
//! of the Step trait for starting the Lotus daemon.

use super::super::step::{Step, StepContext};
use super::container_management::{
    check_existing_container, start_container, wait_for_container_init,
};
use super::prerequisites::{
    check_genesis_and_params, check_image_and_binary, check_ports_availability,
};
use super::setup::{build_docker_command, setup_directories};
use super::verification::{verify_api_connectivity, verify_ports, wait_for_api_file};
use std::error::Error;
use std::path::PathBuf;

/// Step for starting the Lotus execution node
pub struct LotusStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl LotusStep {
    /// Create a new LotusStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }
}

impl Step for LotusStep {
    /// Returns the name of this step
    fn name(&self) -> &str {
        "Start Lotus Daemon"
    }

    /// Performs pre-execution checks before starting the Lotus daemon
    ///
    /// This includes checking for existing containers, verifying port availability,
    /// ensuring required Docker images and binaries exist, and validating
    /// genesis and proof parameter files.
    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        check_existing_container(context)?;
        check_ports_availability()?;
        check_image_and_binary()?;
        check_genesis_and_params()?;
        Ok(())
    }

    /// Executes the main logic to start the Lotus daemon container
    ///
    /// This creates necessary directories, builds the Docker run command with
    /// appropriate volume mounts and port mappings, and starts the container.
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        setup_directories(&self.volumes_dir)?;
        let docker_args = build_docker_command(&self.volumes_dir, context)?;
        start_container(docker_args, context)?;
        Ok(())
    }

    /// Performs post-execution verification after the Lotus daemon starts
    ///
    /// This waits for the container to initialize, verifies port accessibility,
    /// waits for the Lotus API file to be created, and checks API connectivity
    /// including FEVM/Ethereum RPC availability.
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        wait_for_container_init(context)?;
        verify_ports()?;
        wait_for_api_file(&self.volumes_dir)?;
        verify_api_connectivity(context)?;
        Ok(())
    }
}
