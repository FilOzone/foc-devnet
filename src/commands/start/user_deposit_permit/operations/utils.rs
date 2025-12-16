//! Utility functions for user deposit and permit operations.

use super::super::constants::TRANSACTION_CONFIRMATION_WAIT_SECS;

/// Convert token amount to wei (18 decimals)
pub fn token_amount_to_wei(amount_tokens: u64) -> String {
    format!("{}000000000000000000", amount_tokens)
}

/// Wait for transaction confirmation
pub fn wait_for_confirmation() {
    println!(
        "      Waiting {} seconds for transaction confirmation...",
        TRANSACTION_CONFIRMATION_WAIT_SECS
    );
    std::thread::sleep(std::time::Duration::from_secs(
        TRANSACTION_CONFIRMATION_WAIT_SECS,
    ));
}
