//! Constants for Ethereum account funding.
//!
//! This module contains all configuration constants used in the account funding process.

/// Account configuration constants
pub const GLOBAL_FIL_FAUCET_KEY: &str = "GLOBAL_FIL_FAUCET"; // The GLOBAL_FIL_FAUCET account

/// List of FEVM accounts to be prefunded and their respective FIL amounts
pub const FEVM_ACCOUNTS_PREFUNDED: [(&str, u64); 7] = [
    // Contract deployer accounts
    ("DEPLOYER_MOCKUSDFC", 10),
    ("DEPLOYER_FOC", 10),
    ("DEPLOYER_MULTICALL3", 10),
    // Accounts for PDP service providers
    ("PDP_SP_0", 1000),
    // Some user accounts for testing
    ("USER_0", 1000),
    ("USER_1", 1000),
    ("USER_2", 1000),
];

/// Network configuration
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 6;
