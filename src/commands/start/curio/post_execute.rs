//! Post-execution verification for Curio.
//!
//! Verifies that all Curio PDP SPs are working correctly.

use super::super::step::SetupContext;
use super::verification;
use std::error::Error;
use tracing::info;

/// Verify all Curio PDP Service Providers are functioning correctly.
///
/// For each SP:
/// 1. Ping PDP subsystem endpoint
/// 2. Upload a test file
/// 3. Download the file and verify contents
pub fn verify_all_curio_sps(context: &SetupContext, sp_count: usize) -> Result<(), Box<dyn Error>> {
    info!("Verifying all {} Curio PDP SP(s)...", sp_count);

    for sp_index in 1..=sp_count {
        info!("Verifying PDP SP {}...", sp_index);

        verification::verify_single_curio_sp(context, sp_index)?;

        info!("PDP SP {} verification complete", sp_index);
    }

    info!("All {} Curio PDP SP(s) verified successfully", sp_count);

    Ok(())
}
