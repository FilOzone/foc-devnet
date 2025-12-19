//! Genesis file construction and configuration.
//!
//! This module handles creating and modifying the genesis JSON file
//! with signers, miners, and pre-funded accounts.

pub mod accounts;
pub mod creation;
pub mod miner;
pub mod signers;

use crate::paths::foc_localnet_genesis;
use tracing::info;

/// Construct the complete genesis configuration.
///
/// This combines the genesis construction steps:
/// 1. Create initial genesis file
/// 2. Add signers
/// 3. Add miners
/// 4. Add pre-funded accounts (if any)
///
/// If the genesis file already exists, all construction steps are skipped
/// since the genesis configuration is considered complete.
///
/// # Parameters
/// - `active_pdp_sp_count`: Number of active PDP SPs to add to genesis
///
/// # Returns
/// Returns `Ok(())` if genesis is constructed or already exists.
pub fn construct_genesis(
    active_pdp_sp_count: usize,
    run_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let genesis_dir = foc_localnet_genesis(run_id);
    let genesis_file_path = genesis_dir.join(super::constants::GENESIS_FILE);

    // Check if genesis file already exists - if so, skip all construction
    if genesis_file_path.exists() {
        info!(
            "  ✓ Genesis file already exists at {}",
            genesis_file_path.display()
        );
        info!("✓ Genesis construction complete");
        return Ok(());
    }

    info!("Constructing genesis configuration...");

    creation::create_genesis_file(run_id)?;
    signers::add_signers_to_genesis(run_id)?;
    miner::add_miner_to_genesis(active_pdp_sp_count, run_id)?;
    accounts::add_global_fil_faucet_account(run_id)?;
    accounts::add_foc_accounts(run_id)?;

    info!("✓ Genesis construction complete");
    Ok(())
}
