//! Lotus execution node step implementation.
//!
//! This module contains the LotusStep struct and its implementation
//! of the Step trait for starting the Lotus daemon.

use super::super::step::{SetupContext, Step};
use super::container_management::{
    check_existing_container, start_container, wait_for_container_init,
};
use super::prerequisites::check_genesis_and_params;
use super::setup::{build_docker_command, setup_directories};
use super::verification::{verify_api_connectivity, verify_ports, wait_for_api_file};
use std::error::Error;
use std::path::PathBuf;

/// Step for starting the Lotus execution node
pub struct LotusStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    run_dir: PathBuf,
}

impl LotusStep {
    /// Create a new LotusStep
    pub fn new(volumes_dir: PathBuf, run_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            run_dir,
        }
    }
}

impl Step for LotusStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Lotus Daemon"
    }

    /// Perform pre-execution checks
    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        let run_id = context.run_id();
        check_genesis_and_params(run_id)?;

        // Allocate ports for Lotus
        let api_port = context.allocate_port()?;
        let p2p_port = context.allocate_port()?;

        context.set("lotus_api_port", api_port.to_string());
        context.set("lotus_p2p_port", p2p_port.to_string());

        // Check availability
        if !crate::docker::is_port_available(api_port) {
            return Err(format!("Port {} for Lotus API is already in use", api_port).into());
        }
        if !crate::docker::is_port_available(p2p_port) {
            return Err(format!("Port {} for Lotus P2P is already in use", p2p_port).into());
        }

        Ok(())
    }

    /// Execute the Lotus daemon startup process
    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        check_existing_container(context)?;

        setup_directories(&self.volumes_dir)?;
        let docker_args = build_docker_command(&self.volumes_dir, context)?;
        start_container(docker_args, context)?;
        wait_for_container_init(context)?;
        wait_for_api_file(&self.volumes_dir)?;

        Ok(())
    }

    /// Perform post-execution verification for Lotus
    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        verify_ports(context)?;
        verify_api_connectivity(context)
    }
}
