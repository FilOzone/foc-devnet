//! Constants for Ethereum account funding.
//!
//! This module contains all configuration constants used in the account funding process.

/// Account configuration constants
pub const GLOBAL_FIL_FAUCET_KEY: &str = "prefunded-1"; // The GLOBAL_FIL_FAUCET account
pub const FEVM_FAUCET_AMOUNT: &str = "10000"; // 10,000 FIL to transfer to FEVM ecosystem
pub const FOC_DEPLOYER_AMOUNT: &str = "5000"; // 5,000 FIL for contract deployment

/// Network configuration
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 15;