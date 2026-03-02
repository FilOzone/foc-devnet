//! USER_1 payment setup step implementation.
//!
//! After FOC contracts are deployed, this step configures USER_1's wallet for
//! interacting with FOC storage services:
//! - ERC20 approve (USDFC → FilecoinPay)
//! - FilecoinPay deposit
//! - FilecoinPay setOperatorApproval (FWSS as operator)
//!
//! USER_2 and USER_3 are funded with USDFC but not configured for FOC.

use super::constants::POST_SETUP_WAIT_SECONDS;
use super::operations::{load_user_private_key, setup_client_payments};
use crate::commands::start::step::{SetupContext, Step};
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use std::error::Error;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tracing::info;

/// Step that sets up USER_1's wallet for FOC usage via on-chain cast transactions.
pub struct UserSetupStep {
    #[allow(dead_code)]
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    run_dir: PathBuf,
}

impl UserSetupStep {
    /// Create a new UserSetupStep.
    pub fn new(volumes_dir: PathBuf, run_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            run_dir,
        }
    }

    /// Verify that the Lotus container is still running before attempting cast calls.
    fn check_lotus_running(context: &SetupContext) -> Result<(), Box<dyn Error>> {
        let container = lotus_container_name(context.run_id());
        if !container_is_running(&container)? {
            return Err("Lotus container is not running; cannot perform user setup.".into());
        }
        Ok(())
    }
}

impl Step for UserSetupStep {
    fn name(&self) -> &str {
        "USER_1 Payment Setup"
    }

    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        info!("Pre-checking {}", self.name());
        Self::check_lotus_running(context)?;
        info!("Lotus is running");
        Ok(())
    }

    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        info!("Running {}...", self.name());

        let user_key = load_user_private_key()?;
        setup_client_payments(context, &user_key)?;

        info!(
            "Waiting {} seconds for on-chain activation...",
            POST_SETUP_WAIT_SECONDS
        );
        thread::sleep(Duration::from_secs(POST_SETUP_WAIT_SECONDS));

        info!("{} completed", self.name());
        Ok(())
    }

    fn post_execute(&self, _context: &SetupContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
