//! Genesis preparation module for foc-localnet.
//!
//! This module handles one-time setup tasks required before starting the localnet:
//! - Downloading Filecoin proof parameters
//! - Generating BLS keys for lotus
//! - Pre-sealing sectors for genesis block
//!
//! These operations are performed using the foc-builder container and their
//! outputs are cached for reuse across localnet restarts.

use crossterm::style::Stylize;

pub mod constants;
pub mod construction;
pub mod keys;
pub mod proof_parameters;
pub mod sectors;

/// Ensure all genesis prerequisites are prepared.
///
/// This function checks and prepares:
/// 1. Filecoin proof parameters
/// 2. BLS keys (2x)
/// 3. Pre-sealed sectors for 2 miners
/// 4. Genesis file construction
/// 5. Pre-fund accounts (non-miner)
///
/// # Returns
/// Returns `Ok(())` if all prerequisites are ready, or an error if preparation fails.
pub fn ensure_genesis_prerequisites() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Checking genesis prerequisites...".blue().bold());

    // Ensure proof parameters are downloaded
    proof_parameters::ensure_proof_parameters()?;

    // Ensure BLS keys are generated
    keys::ensure_bls_keys()?;

    // Ensure sectors are pre-sealed
    sectors::ensure_presealed_sectors()?;

    // Construct genesis configuration
    construction::construct_genesis()?;

    println!("{}", "✓ All genesis prerequisites are ready".green().bold());
    Ok(())
}
