//! Constants for MockUSDFC token distribution.
//!
//! This module defines constants used in the MockUSDFC funding process.

use ethers_core::types::U256;

/// Convert token amount to wei (multiply by 10^18)
pub fn token_amount_to_wei(tokens: u64) -> U256 {
    U256::from(tokens) * U256::exp10(18)
}

/// Transaction confirmation wait time in seconds
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 5;
