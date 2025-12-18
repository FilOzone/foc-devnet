//! Keys status display for foc-localnet.
//!
//! This module displays the generated addresses and private keys
//! for various foc-localnet components.

use crate::commands::init::keys::load_keys;
use tracing::info;

/// Print the keys status information.
///
/// Displays all generated addresses and their private keys.
pub fn print_keys_status() -> Result<(), Box<dyn std::error::Error>> {
    info!("Generated Keys");

    let keys = load_keys()?;

    for key in keys {
        let addr_display = if let Some(addr) = key.filecoin_address.as_ref() {
            if addr.starts_with("t3") {
                format!("{} (t3)", addr)
            } else if addr.starts_with("t4") {
                format!("{} (t4)", addr)
            } else {
                format!("{} (unknown)", addr)
            }
        } else {
            "N/A".to_string()
        };

        info!("{}: {}", key.name, addr_display);

        if let Some(eth) = key.eth_address {
            info!("  Ethereum: {}", eth);
        }

        info!("  Private Key: {}", key.private_key);
    }

    Ok(())
}
