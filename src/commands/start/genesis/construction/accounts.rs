//! Genesis accounts management.
//!
//! This module handles adding pre-funded accounts and FOC-specific accounts to the genesis file.

use crate::commands::init::keys::load_keys;
use crate::commands::start::genesis::constants;
use crate::commands::start::genesis::keys::get_bls_addresses;
use crate::paths::foc_localnet_genesis;
use crossterm::style::Stylize;
use std::fs;

/// Add pre-funded accounts to the genesis file.
///
/// Since lotus-seed doesn't have an `add-actor` command, we modify the genesis JSON
/// directly to add additional pre-funded accounts that are not signers.
pub fn add_prefunded_accounts() -> Result<(), Box<dyn std::error::Error>> {
    if constants::NUM_PREFUNDED_KEYS == 0 {
        return Ok(());
    }

    println!("  {} Adding pre-funded accounts to genesis...", "💰".cyan());

    let genesis_dir = foc_localnet_genesis();
    let genesis_file_path = genesis_dir.join(constants::GENESIS_FILE);

    // Get pre-funded BLS addresses
    let addresses = get_bls_addresses("prefunded", constants::NUM_PREFUNDED_KEYS)?;

    // Read the genesis file
    let genesis_content = fs::read_to_string(&genesis_file_path)?;
    let mut genesis: serde_json::Value = serde_json::from_str(&genesis_content)?;

    // Add each pre-funded account to the Accounts array
    if let Some(accounts) = genesis.get_mut("Accounts").and_then(|v| v.as_array_mut()) {
        for (i, addr) in addresses.iter().enumerate() {
            // Create account entry with testnet format (t3...)
            let account = serde_json::json!({
                "Type": "account",
                "Balance": "50000000000000000000000",  // 50,000 FIL
                "Meta": {
                    "Owner": format!("t{}", &addr[1..])  // Convert f3... to t3...
                }
            });

            accounts.push(account);
            println!(
                "      {} Pre-funded account {}: {}",
                "✓".green(),
                i + 1,
                addr
            );
        }
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
                        "Balance": "1000000000000000000000000",  // 1,000,000 FIL
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

    // Add evm actors to the Actors array
    if let serde_json::Value::Object(ref mut map) = genesis {
        let actors_value = map
            .entry("Actors")
            .or_insert_with(|| serde_json::Value::Array(vec![]));
        if let serde_json::Value::Array(ref mut actors_array) = actors_value {
            for key in &keys {
                if let (Some(actor_id), Some(eth_addr), Some(fil_addr)) =
                    (key.actor_id, &key.eth_address, &key.filecoin_address)
                {
                    if fil_addr.starts_with("t4") {
                        let actor = serde_json::json!({
                            "ID": actor_id,
                            "Type": "evm",
                            "Balance": "1000000000000000000000000",  // 1,000,000 FIL
                            "Meta": {
                                "DelegatedAddress": fil_addr
                            }
                        });
                        actors_array.push(actor);
                        println!(
                            "      {} Added {}: {} ({})",
                            "✓".green(),
                            key.name,
                            fil_addr,
                            eth_addr
                        );
                    }
                }
            }
        }
    }

    // Write the modified genesis back
    let updated_content = serde_json::to_string_pretty(&genesis)?;
    fs::write(&genesis_file_path, updated_content)?;

    println!("  {} FOC accounts added successfully", "✓".green());
    Ok(())
}
