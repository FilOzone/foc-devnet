//! Constants for Ethereum account funding.
//!
//! This module contains all configuration constants used in the account funding process.

/// Account configuration constants
pub const GLOBAL_FIL_FAUCET_KEY: &str = "prefunded-1"; // The GLOBAL_FIL_FAUCET account

pub const FEVM_ACCOUNTS_PREFUNDED: [(&str, u64); 7] = [
    ("MOCKUSDFC_DEPLOYER", 1000),
    ("FOC_DEPLOYER", 1000),
    ("MULTICALL3_DEPLOYER", 1000),
    ("PDP_SP_1", 1000),
    ("USER_1", 1000),
    ("USER_2", 1000),
    ("USER_3", 1000),
];

/// Network configuration
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 6;
