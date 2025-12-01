//! Keys status display for foc-localnet.
//!
//! This module displays the generated addresses and private keys
//! for various foc-localnet components.

use crate::commands::init::keys::load_keys;
use crossterm::style::Stylize;

/// Print the keys status information.
///
/// Displays all generated addresses and their private keys.
pub fn print_keys_status() -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("{}", "🔑 Generated Keys".bold().cyan());
    println!("{}", "─".repeat(80).cyan());

    let keys = load_keys()?;

    for key in keys {
        println!("{}: {}", key.name.bold(), {
            if let Some(addr) = key.filecoin_address.as_ref() {
                if addr.starts_with("t3") {
                    format!("{} (t3)", addr)
                } else if addr.starts_with("t4") {
                    format!("{} (t4)", addr)
                } else {
                    format!("{} (unknown)", addr)
                }
            } else {
                "N/A".to_string()
            }
        });

        if let Some(eth) = key.eth_address {
            println!("  Ethereum: {}", eth);
        }

        println!("  Private Key: {}", key.private_key.dim());

        println!();
    }

    Ok(())
}
