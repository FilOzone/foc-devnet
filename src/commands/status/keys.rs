//! Keys status display for foc-localnet.
//!
//! This module displays the generated addresses and where to find their
//! private keys for various foc-localnet components.
use crate::paths::foc_localnet_keys;
use tracing::info;

/// Print the keys status information.
pub fn print_keys_status() -> Result<(), Box<dyn std::error::Error>> {
    let keys_dir = foc_localnet_keys();
    info!("Keys stored in: {}", keys_dir.display());

    Ok(())
}
