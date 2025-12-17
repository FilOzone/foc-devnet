//! Post-execution verification for Curio.
//!
//! Verifies that all Curio PDP SPs are working correctly.

use super::super::step::StepContext;
use super::verification;
use crossterm::style::Stylize;
use std::error::Error;

/// Verify all Curio PDP Service Providers are functioning correctly.
///
/// For each SP:
/// 1. Ping PDP subsystem endpoint
/// 2. Upload a test file
/// 3. Download the file and verify contents
pub fn verify_all_curio_sps(context: &StepContext, sp_count: usize) -> Result<(), Box<dyn Error>> {
    println!(
        "  {} Verifying all {} Curio PDP SP(s)...",
        "🔍".cyan(),
        sp_count
    );

    for sp_index in 1..=sp_count {
        println!("    {} Verifying PDP SP {}...", "🔍".cyan(), sp_index);

        verification::verify_single_curio_sp(context, sp_index)?;

        println!(
            "    {} PDP SP {} verification complete",
            "✓".green(),
            sp_index
        );
    }

    println!(
        "  {} All {} Curio PDP SP(s) verified successfully",
        "✓".green(),
        sp_count
    );

    Ok(())
}
