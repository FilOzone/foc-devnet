//! Curio multi-SP step implementation.
//!
//! This module handles the complete setup and verification of Curio PDP Service Providers.
//! It supports multiple isolated Curio instances, each with their own database and storage.

pub mod constants;
pub mod db_setup;
pub mod daemon;
pub mod pdp;
pub mod pre_execute;
pub mod execute;
pub mod post_execute;
pub mod storage;
pub mod verification;

use super::step::{Step, StepContext};
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;

/// Step for setting up Curio PDP Service Providers.
///
/// This step:
/// - Verifies Lotus is running and producing blocks
/// - Sets up Yugabyte database for each Curio SP
/// - Configures base and PDP layers
/// - Starts Curio daemon with appropriate layers
/// - Attaches storage locations
/// - Imports PDP private keys
/// - Verifies PDP subsystem and upload/download functionality
pub struct CurioStep {
    #[allow(dead_code)]
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
    /// Number of PDP SPs to activate (1-5)
    active_sp_count: usize,
}

impl CurioStep {
    /// Create a new CurioStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        // TODO: Read from config.toml
        let active_sp_count = crate::commands::start::genesis::constants::ACTIVE_PDP_SP_COUNT;
        
        Self {
            volumes_dir,
            logs_dir,
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

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("{}", format!("Pre-checks for {}", self.name()).blue().bold());
        pre_execute::verify_prerequisites(context, self.active_sp_count)?;
        Ok(())
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("{}", format!("Executing {}", self.name()).blue().bold());
        execute::setup_all_curio_sps(context, self)?;
        Ok(())
    }

    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("{}", format!("Verifying {}", self.name()).blue().bold());
        post_execute::verify_all_curio_sps(context, self.active_sp_count)?;
        Ok(())
    }

    fn run(&self, context: &mut StepContext) -> Result<std::time::Duration, Box<dyn Error>> {
        let start = std::time::Instant::now();
        self.pre_execute(context)?;
        self.execute(context)?;
        self.post_execute(context)?;
        println!("{}", format!("✓ {} completed successfully", self.name()).green().bold());
        Ok(start.elapsed())
    }
}
