//! Constants for MockUSDFC token distribution.
//!
//! This module defines constants used in the MockUSDFC funding process.

/// Amount of MockUSDFC tokens to distribute to each user (100,000 tokens)
pub const USER_TOKEN_AMOUNT: &str = "100000000000000000000000"; // 100,000 * 10^18

/// Amount of MockUSDFC tokens to distribute to PDP_SP_0 (200,000 tokens)
pub const PDP_SP_TOKEN_AMOUNT: &str = "200000000000000000000000"; // 200,000 * 10^18

/// Transaction confirmation wait time in seconds
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 5;