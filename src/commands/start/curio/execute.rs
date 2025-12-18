//! Main execution logic for Curio setup.
//!
//! Orchestrates the setup of all Curio PDP Service Providers.

use super::super::step::StepContext;
use super::daemon;
use super::db_setup;
use super::pdp;
use super::storage;
use super::CurioStep;
use crossterm::style::Stylize;
use std::error::Error;

/// Setup all Curio PDP Service Providers.
///
/// For each SP (1 through active_sp_count):
/// 1. Setup database (base layer and PDP layer)
/// 2. Start Curio daemon
/// 3. Attach storage locations
/// 4. Import PDP private key
pub fn setup_all_curio_sps(
    context: &mut StepContext,
    step: &CurioStep,
) -> Result<(), Box<dyn Error>> {
    for sp_index in 1..=step.active_sp_count() {
        println!("  {} Setting up Curio PDP SP {}...", "🚀".cyan(), sp_index);

        setup_single_curio_sp(context, step, sp_index)?;

        println!("  {} Curio PDP SP {} setup complete", "✓".green(), sp_index);
    }

    println!(
        "  {} All {} Curio PDP SP(s) setup successfully",
        "✓".green(),
        step.active_sp_count()
    );

    Ok(())
}

/// Setup a single Curio PDP Service Provider.
fn setup_single_curio_sp(
    context: &mut StepContext,
    step: &CurioStep,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Setup database (base layer migration + PDP layer config)
    db_setup::setup_curio_database(context, sp_index)?;

    // Step 2: Start Curio daemon
    daemon::start_curio_daemon(context, step, sp_index)?;

    // Step 3: Attach storage locations
    storage::attach_storage_locations(context, sp_index)?;

    // Step 4: Import PDP private key
    pdp::import_pdp_key(context, sp_index)?;

    Ok(())
}
