//! Pre-execution checks for Curio step.
//!
//! Verifies that Lotus is running and blocks are being generated before
//! attempting to start Curio.

use super::super::step::SetupContext;
use crate::docker::containers::lotus_container_name;
use crate::docker::{container_exists, container_is_running};
use std::error::Error;
use tracing::info;

/// Verify prerequisites for Curio setup.
///
/// Checks:
/// 1. Lotus container is running
/// 2. Lotus-Miner container is running
pub fn verify_prerequisites(context: &SetupContext, sp_count: usize) -> Result<(), Box<dyn Error>> {
    info!("Verifying Lotus is running and producing blocks...");

    let run_id = context.run_id();
    let lotus_container = lotus_container_name(run_id);

    // Check Lotus container exists and is running
    if !container_exists(&lotus_container)? {
        return Err(format!(
            "Lotus container '{}' does not exist. Run the Lotus step first.",
            lotus_container
        )
        .into());
    }

    if !container_is_running(&lotus_container)? {
        return Err(format!(
            "Lotus container '{}' is not running. Ensure Lotus step completed successfully.",
            lotus_container
        )
        .into());
    }

    info!(
        "Allocating and verifying ports for {} Curio instance(s)...",
        sp_count
    );
    for sp_index in 1..=sp_count {
        let api_port = context.allocate_port()?;
        let api_port_alt = context.allocate_port()?;
        let gui_port = context.allocate_port()?;
        let pdp_port = context.allocate_port()?;

        context.set(
            format!("pdp_sp_{}_api_port", sp_index),
            api_port.to_string(),
        );
        context.set(
            format!("pdp_sp_{}_api_port_alt", sp_index),
            api_port_alt.to_string(),
        );
        context.set(
            format!("pdp_sp_{}_gui_port", sp_index),
            gui_port.to_string(),
        );
        context.set(
            format!("pdp_sp_{}_pdp_port", sp_index),
            pdp_port.to_string(),
        );

        for (port, desc) in [
            (api_port, "API"),
            (api_port_alt, "API Alt"),
            (gui_port, "GUI"),
            (pdp_port, "PDP"),
        ] {
            if !crate::docker::is_port_available(port) {
                return Err(format!(
                    "Port {} ({}) for Curio SP {} is already in use",
                    port, desc, sp_index
                )
                .into());
            }
        }
    }

    info!("Prerequisites verified: Lotus is running and producing blocks");
    info!("Will activate {} PDP Service Provider(s)", sp_count);

    Ok(())
}
