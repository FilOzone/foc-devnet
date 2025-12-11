//! Genesis file construction and configuration.
//!
//! This module handles creating and modifying the genesis JSON file
//! with signers, miners, and pre-funded accounts.

pub mod accounts;
pub mod creation;
pub mod miner;
pub mod signers;

use crate::paths::foc_localnet_genesis;
use crossterm::style::Stylize;

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
pub fn construct_genesis() -> Result<(), Box<dyn std::error::Error>> {
    let genesis_dir = foc_localnet_genesis();
    let genesis_file_path = genesis_dir.join(super::constants::GENESIS_FILE);

    // Check if genesis file already exists - if so, skip all construction
    if genesis_file_path.exists() {
        println!(
            "  {} Genesis file already exists at {}",
            "✓".green(),
            genesis_file_path.display()
        );
        println!("{}", "✓ Genesis construction complete".green().bold());
        return Ok(());
    }

    println!("{}", "Constructing genesis configuration...".blue().bold());

    creation::create_genesis_file()?;
    signers::add_signers_to_genesis()?;
    miner::add_miner_to_genesis()?;
    accounts::add_global_fil_faucet_account()?;
    accounts::add_foc_accounts()?;

    println!("{}", "✓ Genesis construction complete".green().bold());
    Ok(())
}
