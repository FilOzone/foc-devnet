//! Cryptographic key generation and address computation utilities.
//!
//! This module provides low-level cryptographic functions for generating
//! deterministic keys and computing Filecoin/Ethereum addresses.

pub mod bls;
pub mod ethereum;
pub mod mnemonic;

/// Derived key information containing private key and addresses.
#[derive(Debug, Clone)]
pub struct DerivedKey {
    /// Hex-encoded private key
    pub private_key: String,
    /// Filecoin address (for BLS and Ethereum style keys)
    pub native_address: String,
    /// Ethereum address (for Ethereum keys)
    pub eth_address: Option<String>,
}

// Re-export functions for convenience
pub use bls::{compute_bls_address, derive_bls_key};
pub use ethereum::{compute_eth_address, compute_native_address, derive_ethereum_key};
