//! Key generation and management for foc-localnet addresses.
//!
//! This module handles generating deterministic addresses and private keys
//! for various components of the foc-localnet system using HD wallet derivation.

use crate::{
    commands::start::FEVM_ACCOUNTS_PREFUNDED, crypto::mnemonic::store_mnemonic,
    paths::foc_localnet_keys,
};
use bip39::{Language, Mnemonic};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use tracing::info;

/// Information about a generated key and its addresses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    /// Name of the key (e.g., "GLOBAL_FIL_FAUCET")
    pub name: String,
    /// Hex-encoded private key
    pub private_key: String,
    /// Filecoin address (t3 or t4)
    pub filecoin_address: Option<String>,
    /// Ethereum address (only for t4 addresses)
    pub eth_address: Option<String>,
}

impl fmt::Display for KeyInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Key: {}", self.name)?;
        if let Some(filecoin_addr) = &self.filecoin_address {
            writeln!(f, "  Filecoin Address: {}", filecoin_addr)?;
        }
        if let Some(eth_addr) = &self.eth_address {
            writeln!(f, "  Ethereum Address:  {}", eth_addr)?;
        }
        writeln!(f, "  Private Key:       {}", self.private_key)?;
        Ok(())
    }
}

pub const STATIC_MNEMONIC_ENTROPY: [u8; 32] = [
    0x73, 0x75, 0x54, 0x32, 0x65, 0x8a, 0x20, 0x73, 0x11, 0x85, 0x6e, 0x04, 0x20, 0x6d, 0x61, 0x77,
    0x6b, 0x20, 0x44, 0x1f, 0x4b, 0x65, 0x29, 0x56, 0x69, 0x62, 0x72, 0x31, 0x63, 0x34, 0x27, 0x53,
];

pub const NATIVE_KEYS: [&str; 3] = ["BLS_SIGNER_1", "BLS_SIGNER_2", "GLOBAL_FIL_FAUCET"];

/// Generate all required keys for foc-localnet.
///
/// This function generates keys for:
/// - BLS_SIGNER_1 (t3 address)
/// - BLS_SIGNER_2 (t3 address)
/// - GLOBAL_FIL_FAUCET (t3 address)
///
/// Keys are derived deterministically from a static mnemonic.
/// If use_random is true, a random mnemonic is used instead.
pub fn generate_keys(use_random: bool) -> Result<Vec<KeyInfo>, Box<dyn std::error::Error>> {
    let mnemonic_entropy = match use_random {
        true => {
            info!("Using system randomness for generating mnemonic for deterministic addresses");
            let entropy: [u8; 32] = rand::random();
            entropy
        }
        false => {
            info!("Using deterministic mnemonic for addresses");
            STATIC_MNEMONIC_ENTROPY
        }
    };

    let mnemonic = Mnemonic::from_entropy_in(Language::English, &mnemonic_entropy)?;
    store_mnemonic(&mnemonic)?;
    let seed = mnemonic.to_seed("");
    info!("Mnemonic: {}", mnemonic);

    let mut keys = Vec::with_capacity(NATIVE_KEYS.len() + FEVM_ACCOUNTS_PREFUNDED.len());

    for key_name in &NATIVE_KEYS {
        let bls_key = crate::crypto::derive_bls_key(&seed, key_name)?;
        keys.push(KeyInfo {
            name: key_name.to_string(),
            private_key: bls_key.private_key,
            filecoin_address: Some(bls_key.native_address),
            eth_address: None,
        });
    }

    for (key_name, _) in &FEVM_ACCOUNTS_PREFUNDED {
        let eth_key = crate::crypto::derive_ethereum_key(&seed, key_name)?;
        keys.push(KeyInfo {
            name: key_name.to_string(),
            private_key: eth_key.private_key,
            filecoin_address: Some(eth_key.native_address),
            eth_address: eth_key.eth_address,
        });
    }

    // Pretty print all generated keys
    info!("Generated Keys:");
    for key in &keys {
        info!(
            "    - {}: {} private key: ({})",
            key.name,
            key.filecoin_address.as_deref().unwrap_or("N/A"),
            key.private_key
        );
    }

    // Save keys to file
    save_keys(&keys)?;

    Ok(keys)
}

/// Save generated keys to ~/.foc-localnet/keys/addresses.json
fn save_keys(keys: &[KeyInfo]) -> Result<(), Box<dyn std::error::Error>> {
    let keys_dir = foc_localnet_keys();
    fs::create_dir_all(&keys_dir)?;
    let keys_file = keys_dir.join("addresses.json");
    let json = serde_json::to_string_pretty(keys)?;
    fs::write(keys_file, json)?;
    info!("Keys saved to {}", keys_dir.display());
    Ok(())
}

/// Load keys from ~/.foc-localnet/keys/addresses.json
pub fn load_keys() -> Result<Vec<KeyInfo>, Box<dyn std::error::Error>> {
    let keys_dir = foc_localnet_keys();
    let keys_file = keys_dir.join("addresses.json");
    let json = fs::read_to_string(keys_file)?;
    let keys: Vec<KeyInfo> = serde_json::from_str(&json)?;
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keys_includes_bls_keys() {
        let keys = generate_keys(false).unwrap();

        // Check that we have the expected number of keys
        assert_eq!(keys.len(), 8); // 3 BLS + 5 Ethereum

        // Check that BLS keys are present
        let bls_keys: Vec<_> = keys
            .iter()
            .filter(|k| k.name.starts_with("BLS_") || k.name == "GLOBAL_FIL_FAUCET")
            .collect();
        assert_eq!(bls_keys.len(), 3);

        // Check that BLS keys have t3 addresses
        for key in &bls_keys {
            assert!(key.filecoin_address.as_ref().unwrap().starts_with("t3"));
            assert!(key.eth_address.is_none());
        }

        // Check specific key names
        let key_names: Vec<_> = keys.iter().map(|k| k.name.as_str()).collect();
        assert!(key_names.contains(&"BLS_SIGNER_1"));
        assert!(key_names.contains(&"BLS_SIGNER_2"));
        assert!(key_names.contains(&"GLOBAL_FIL_FAUCET"));
    }

    #[test]
    fn test_derive_bls_key() {
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, "sudden spend mask joke vibrant situate tilt history occur rally artwork shadow gather proud urban work own quick holiday bone target zone unknown nut").unwrap();
        let seed = mnemonic.to_seed("");

        let key = crate::crypto::derive_bls_key(&seed, "test_key").unwrap();

        // Check that we get a valid t3 address
        assert!(key.native_address.starts_with("t3"));
        // t3 addresses can vary in length due to base32 encoding, but should be reasonable length
        assert!(key.native_address.len() > 10 && key.native_address.len() < 100);

        // Check that private key hex string is valid (BLS private key is 32 bytes = 64 hex chars)
        assert!(hex::decode(&key.private_key).is_ok());
        let pk_bytes = hex::decode(&key.private_key).unwrap();
        assert_eq!(pk_bytes.len(), 32);

        // Check that eth_address is empty for BLS
        assert!(key.eth_address.is_none());
    }
}
