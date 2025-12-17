//! Constants for MockUSDFC token distribution.
//!
//! This module defines constants used in the MockUSDFC funding process.

/// List of accounts to be funded with MockUSDFC tokens and their respective amounts (in tokens)
pub const USDFC_ACCOUNTS_FUNDED: [(&str, u64); 8] = [
    // User accounts for testing
    ("USER_0", 100_000),
    ("USER_1", 100_000),
    ("USER_2", 100_000),
    // PDP service provider accounts (base-1 numbering)
    ("PDP_SP_1", 200_000),
    ("PDP_SP_2", 200_000),
    ("PDP_SP_3", 200_000),
    ("PDP_SP_4", 200_000),
    ("PDP_SP_5", 200_000),
];

/// Convert token amount to wei (multiply by 10^18)
pub fn token_amount_to_wei(tokens: u64) -> String {
    format!("{}000000000000000000", tokens) // tokens * 10^18
}

/// Transaction confirmation wait time in seconds
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 5;
