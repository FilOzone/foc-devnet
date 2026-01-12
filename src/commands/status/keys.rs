//! Keys status display for foc-devnet.
//!
//! This module displays the generated addresses and where to find their
//! private keys for various foc-devnet components.
use crate::paths::foc_devnet_keys;
use tracing::info;

/// Print the keys status information.
pub fn print_keys_status() -> Result<(), Box<dyn std::error::Error>> {
    let keys_dir = foc_devnet_keys();
    info!(
        "Deterministic Keys and Addresses stored in: {}",
        keys_dir.display()
    );

    Ok(())
}
