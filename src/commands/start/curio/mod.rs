//! Curio multi-SP step implementation.
//!
//! This module handles the complete setup and verification of Curio PDP Service Providers.
//! It supports multiple isolated Curio instances, each with their own database and storage.

pub mod constants;
pub mod daemon;
pub mod db_setup;
pub mod execute;
pub mod pdp;
pub mod post_execute;
pub mod pre_execute;
pub mod storage;
pub mod verification;

use super::step::{SetupContext, Step};
use std::error::Error;
use std::path::PathBuf;
use tracing::info;

/// Step for setting up Curio PDP Service Providers.
///
/// This step:
/// - Verifies Lotus is running and producing blocks
/// - Runs database migrations against each Curio SP's Postgres
/// - Configures base and PDP layers
/// - Starts Curio daemon with appropriate layers
/// - Attaches storage locations
/// - Imports PDP private keys
/// - Verifies PDP subsystem and upload/download functionality
pub struct CurioStep {
    #[allow(dead_code)]
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    run_dir: PathBuf,
    /// Number of PDP SPs to activate (1-5)
    active_sp_count: usize,
}

impl CurioStep {
    /// Create a new CurioStep
    pub fn new(volumes_dir: PathBuf, run_dir: PathBuf, active_sp_count: usize) -> Self {
        Self {
            volumes_dir,
            run_dir,
            active_sp_count,
        }
    }

    /// Get the number of active PDP SPs
    pub fn active_sp_count(&self) -> usize {
        self.active_sp_count
    }
}

impl Step for CurioStep {
    fn name(&self) -> &str {
        "Curio PDP Service Providers"
    }

    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        info!("Pre-checks for {}...", self.name());
        pre_execute::verify_prerequisites(context, self.active_sp_count)?;
        Ok(())
    }

    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        info!("Executing {}...", self.name());
        execute::setup_all_curio_sps(context, self)?;
        Ok(())
    }

    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        info!("Verifying {}...", self.name());
        post_execute::verify_all_curio_sps(context, self.active_sp_count)?;
        Ok(())
    }

    fn run(&self, context: &SetupContext) -> Result<std::time::Duration, Box<dyn Error>> {
        let start = std::time::Instant::now();
        self.pre_execute(context)?;
        self.execute(context)?;
        self.post_execute(context)?;
        info!("✓ {} completed successfully", self.name());
        Ok(start.elapsed())
    }
}
