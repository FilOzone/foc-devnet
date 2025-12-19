//! Main execution logic for Curio setup.
//!
//! Orchestrates the setup of all Curio PDP Service Providers.

use super::super::step::SetupContext;
use super::daemon;
use super::db_setup;
use super::pdp;
use super::storage;
use super::CurioStep;
use std::error::Error;
use std::thread;
use tracing::info;

/// Setup all Curio PDP Service Providers.
///
/// For each SP (1 through active_sp_count):
/// 1. Setup database (base layer and PDP layer)
/// 2. Start Curio daemon
/// 3. Attach storage locations
/// 4. Import PDP private key
pub fn setup_all_curio_sps(context: &SetupContext, step: &CurioStep) -> Result<(), Box<dyn Error>> {
    thread::scope(|s| {
        let mut handles = vec![];

        for sp_index in 1..=step.active_sp_count() {
            handles.push(s.spawn(move || {
                info!("Setting up Curio PDP SP {}...", sp_index);

                setup_single_curio_sp(context, step, sp_index)
                    .map_err(|e| format!("SP {}: {}", sp_index, e))?;

                info!("Curio PDP SP {} setup complete", sp_index);
                Ok::<(), String>(())
            }));
        }

        let mut errors = vec![];
        for handle in handles {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(e),
                Err(_) => errors.push("Thread panicked".to_string()),
            }
        }

        if !errors.is_empty() {
            let err_msg = format!("Parallel setup failed: {}", errors.join("; "));
            return Err(Box::<dyn Error>::from(err_msg));
        }

        Ok(())
    })?;

    info!(
        "All {} Curio PDP SP(s) setup successfully",
        step.active_sp_count()
    );

    Ok(())
}

/// Setup a single Curio PDP Service Provider.
fn setup_single_curio_sp(
    context: &SetupContext,
    step: &CurioStep,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    // Step 1: Setup database (base layer migration + PDP layer config)
    db_setup::setup_curio_database(context, sp_index)?;

    // Step 2: Start Curio daemon
    daemon::start_curio_daemon(context, step, sp_index)?;

    // Wait a bit to ensure daemon is fully started before proceeding
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Step 3: Attach storage locations
    storage::attach_storage_locations(context, sp_index)?;

    // Step 4: Import PDP private key
    pdp::import_pdp_key(context, sp_index)?;

    Ok(())
}
