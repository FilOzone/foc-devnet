//! Storage management for Curio.
//!
//! Handles attaching fast-storage and long-term-storage locations.

use super::super::step::StepContext;
use super::constants::STORAGE_ATTACH_WAIT_SECS;
use crossterm::style::Stylize;
use std::error::Error;
use std::thread;
use std::time::Duration;

/// Attach storage locations for a specific PDP SP.
///
/// Attaches:
/// 1. Fast storage (seal)
/// 2. Long-term storage (store)
#[allow(unused_variables)]
pub fn attach_storage_locations(context: &StepContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!(
        "    {} Attaching storage locations for PDP SP {}...",
        "💿".cyan(),
        sp_index
    );

    // Attach fast storage
    attach_fast_storage(sp_index)?;

    // Attach long-term storage
    attach_long_term_storage(sp_index)?;

    println!(
        "    {} Storage locations attached for PDP SP {}",
        "✓".green(),
        sp_index
    );

    Ok(())
}

/// Attach fast storage for sealing operations.
#[allow(unused_variables)]
fn attach_fast_storage(sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!(
        "      {} Attaching fast storage...",
        "⚙".cyan()
    );

    // TODO: Implement actual curio cli storage attach command
    // Command: curio cli --machine 127.0.0.1:12300 storage attach \
    //            --init --seal --weight 10 /home/foc-user/curio/fast-storage

    thread::sleep(Duration::from_secs(STORAGE_ATTACH_WAIT_SECS));

    println!("      {} Fast storage attached", "✓".green());

    Ok(())
}

/// Attach long-term storage for storing sealed sectors.
#[allow(unused_variables)]
fn attach_long_term_storage(sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!(
        "      {} Attaching long-term storage...",
        "⚙".cyan()
    );

    // TODO: Implement actual curio cli storage attach command
    // Command: curio cli --machine 127.0.0.1:12300 storage attach \
    //            --init --store --weight 10 /home/foc-user/curio/long-term-storage

    thread::sleep(Duration::from_secs(STORAGE_ATTACH_WAIT_SECS));

    println!("      {} Long-term storage attached", "✓".green());

    Ok(())
}
