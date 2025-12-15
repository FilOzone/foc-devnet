//! Genesis accounts management.
//!
//! This module handles adding pre-funded accounts and FOC-specific accounts to the genesis file.

use crate::commands::init::keys::load_keys;
use crate::commands::start::genesis::constants;
use crate::commands::start::genesis::keys::get_bls_addresses;
use crate::paths::foc_localnet_genesis;
use crossterm::style::Stylize;
use std::fs;

/// Initial balance for FOC-specific accounts in FIL (without decimals).
pub const PREFUNDED_ACCOUNTS_INIT_FIL: u64 = 10_000_000; // 10 million FIL

/// Add pre-funded accounts to the genesis file.
///
/// Since lotus-seed doesn't have an `add-actor` command, we modify the genesis JSON
/// directly to add the GLOBAL_FIL_FAUCET pre-funded account.
pub fn add_global_fil_faucet_account() -> Result<(), Box<dyn std::error::Error>> {
    println!("  {} Adding pre-funded accounts to genesis...", "💰".cyan());

    let genesis_dir = foc_localnet_genesis();
    let genesis_file_path = genesis_dir.join(constants::GENESIS_FILE);

    // Get GLOBAL_FIL_FAUCET BLS address
    let addresses = get_bls_addresses("GLOBAL_FIL_FAUCET", 0)?;
    let global_fil_faucet_addr = &addresses[0];

    // Read the genesis file
    let genesis_content = fs::read_to_string(&genesis_file_path)?;
    let mut genesis: serde_json::Value = serde_json::from_str(&genesis_content)?;

    // Add GLOBAL_FIL_FAUCET to the Accounts array
    if let Some(accounts) = genesis.get_mut("Accounts").and_then(|v| v.as_array_mut()) {
        // Create account entry with testnet format (t3...)
        let account = serde_json::json!({
            "Type": "account",
            "Balance": format!("{}0000000000000000000", PREFUNDED_ACCOUNTS_INIT_FIL),
            "Meta": {
                "Owner": format!("t{}", &global_fil_faucet_addr[1..])  // Convert f3... to t3...
            }
        });

        accounts.push(account);
        println!(
            "      {} GLOBAL_FIL_FAUCET: {}",
            "✓".green(),
            global_fil_faucet_addr
        );
    } else {
        return Err("Genesis file does not have an 'Accounts' array".into());
    }

    // Write the modified genesis back
    let updated_content = serde_json::to_string_pretty(&genesis)?;
    fs::write(&genesis_file_path, updated_content)?;

    println!("  {} Pre-funded accounts added successfully", "✓".green());
    Ok(())
}

/// Add FOC-specific accounts to the genesis file.
///
/// This includes GLOBAL_FIL_FAUCET as a t3 account and FEVM addresses as evm actors.
pub fn add_foc_accounts() -> Result<(), Box<dyn std::error::Error>> {
    println!("  {} Adding FOC accounts to genesis...", "💰".cyan());

    let genesis_dir = foc_localnet_genesis();
    let genesis_file_path = genesis_dir.join(constants::GENESIS_FILE);

    // Load keys
    let keys = load_keys()?;

    // Read the genesis file
    let genesis_content = fs::read_to_string(&genesis_file_path)?;
    let mut genesis: serde_json::Value = serde_json::from_str(&genesis_content)?;

    // Add accounts to the Accounts array
    if let Some(accounts) = genesis.get_mut("Accounts").and_then(|v| v.as_array_mut()) {
        for key in &keys {
            if let Some(fil_addr) = &key.filecoin_address {
                if fil_addr.starts_with("t3") {
                    // Add t3 account
                    let account = serde_json::json!({
                        "Type": "account",
                        "Balance": format!("{}0000000000000000000", PREFUNDED_ACCOUNTS_INIT_FIL),
                        "Meta": {
                            "Owner": fil_addr
                        }
                    });
                    accounts.push(account);
                    println!("      {} Added {}: {}", "✓".green(), key.name, fil_addr);
                }
            }
        }
    } else {
        return Err("Genesis file does not have an 'Accounts' array".into());
    }

    // Write the modified genesis back
    let updated_content = serde_json::to_string_pretty(&genesis)?;
    fs::write(&genesis_file_path, updated_content)?;

    println!("  {} FOC accounts added successfully", "✓".green());
    Ok(())
}
