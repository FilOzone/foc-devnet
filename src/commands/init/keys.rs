//! Key generation and management for foc-localnet addresses.
//!
//! This module handles generating deterministic addresses and private keys
//! for various components of the foc-localnet system using HD wallet derivation.

use crate::paths::foc_localnet_keys;
use bip39::{Language, Mnemonic};
use crossterm::style::Stylize;
use serde::{Deserialize, Serialize};
use std::fs;

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
    /// Actor ID (only for t4 addresses with assigned IDs)
    pub actor_id: Option<u64>,
}

/// Generate all required keys for foc-localnet.
///
/// This function generates keys for:
/// - BLS_SIGNER_1 (t3 address)
/// - BLS_SIGNER_2 (t3 address)
/// - GLOBAL_FIL_FAUCET (t3 address, same as BLS prefunded-1)
/// - FEVM_FAUCET (t4 address)
/// - MOCKUSDFC_DEPLOYER (t4 address)
/// - FOC_DEPLOYER (t4 address)
/// - PDP_SP1 (t4 address)
/// - USER_ADDRESS (t4 address)
///
/// Keys are derived deterministically from a static mnemonic.
/// If use_random is true, a random mnemonic is used instead.
pub fn generate_keys(use_random: bool) -> Result<Vec<KeyInfo>, Box<dyn std::error::Error>> {
    let mnemonic = if use_random {
        println!("  {} Generating random mnemonic for deterministic addresses", "🔑".cyan());
        let entropy: [u8; 32] = rand::random();
        Mnemonic::from_entropy_in(Language::English, &entropy)?
    } else {
        println!("  {} Using deterministic mnemonic for addresses", "🔑".cyan());
        let static_mnemonic = "sudden spend mask joke vibrant situate tilt history occur rally artwork shadow gather proud urban work own quick holiday bone target zone unknown nut";
        Mnemonic::parse_in_normalized(Language::English, static_mnemonic)?
    };

    println!("  {} Mnemonic: {}", "🔑".cyan(), mnemonic);

    let seed = mnemonic.to_seed("");

    let mut keys = Vec::new();

    // BLS signer keys: Filecoin BLS (coin 461)
    let bls_signer_1 = crate::crypto::derive_bls_key(&seed, 0)?;
    keys.push(KeyInfo {
        name: "BLS_SIGNER_1".to_string(),
        private_key: bls_signer_1.private_key,
        filecoin_address: Some(bls_signer_1.t3_address),
        eth_address: None,
        actor_id: None,
    });

    let bls_signer_2 = crate::crypto::derive_bls_key(&seed, 1)?;
    keys.push(KeyInfo {
        name: "BLS_SIGNER_2".to_string(),
        private_key: bls_signer_2.private_key,
        filecoin_address: Some(bls_signer_2.t3_address),
        eth_address: None,
        actor_id: None,
    });

    // GLOBAL_FIL_FAUCET: BLS prefunded key (coin 461)
    let global_fil_faucet = crate::crypto::derive_bls_key(&seed, 2)?;
    keys.push(KeyInfo {
        name: "GLOBAL_FIL_FAUCET".to_string(),
        private_key: global_fil_faucet.private_key,
        filecoin_address: Some(global_fil_faucet.t3_address),
        eth_address: None,
        actor_id: None,
    });

    // FEVM addresses: Ethereum derivation (coin 60)
    let fevm_faucet = crate::crypto::derive_ethereum_key(&seed, 0)?;
    keys.push(KeyInfo {
        name: "FEVM_FAUCET".to_string(),
        private_key: fevm_faucet.private_key,
        filecoin_address: Some(crate::crypto::compute_t4_address(&fevm_faucet.eth_address, 10)?),
        eth_address: Some(fevm_faucet.eth_address),
        actor_id: Some(1),
    });

    let mock_usdfc_deployer = crate::crypto::derive_ethereum_key(&seed, 1)?;
    keys.push(KeyInfo {
        name: "MOCKUSDFC_DEPLOYER".to_string(),
        private_key: mock_usdfc_deployer.private_key,
        filecoin_address: Some(crate::crypto::compute_t4_address(&mock_usdfc_deployer.eth_address, 10)?),
        eth_address: Some(mock_usdfc_deployer.eth_address),
        actor_id: Some(2),
    });

    let foc_deployer = crate::crypto::derive_ethereum_key(&seed, 2)?;
    keys.push(KeyInfo {
        name: "FOC_DEPLOYER".to_string(),
        private_key: foc_deployer.private_key,
        filecoin_address: Some(crate::crypto::compute_t4_address(&foc_deployer.eth_address, 10)?),
        eth_address: Some(foc_deployer.eth_address),
        actor_id: Some(3),
    });

    let pdp_sp1 = crate::crypto::derive_ethereum_key(&seed, 3)?;
    keys.push(KeyInfo {
        name: "PDP_SP1".to_string(),
        private_key: pdp_sp1.private_key,
        filecoin_address: Some(crate::crypto::compute_t4_address(&pdp_sp1.eth_address, 10)?),
        eth_address: Some(pdp_sp1.eth_address),
        actor_id: Some(4),
    });

    let user_address = crate::crypto::derive_ethereum_key(&seed, 4)?;
    keys.push(KeyInfo {
        name: "USER_ADDRESS".to_string(),
        private_key: user_address.private_key,
        filecoin_address: Some(crate::crypto::compute_t4_address(&user_address.eth_address, 10)?),
        eth_address: Some(user_address.eth_address),
        actor_id: Some(5),
    });

    // Save keys to file
    save_keys(&keys)?;

    Ok(keys)
}

/// Save keys to JSON file.
fn save_keys(keys: &[KeyInfo]) -> Result<(), Box<dyn std::error::Error>> {
    let keys_dir = foc_localnet_keys();
    fs::create_dir_all(&keys_dir)?;
    let keys_file = keys_dir.join("addresses.json");
    let json = serde_json::to_string_pretty(keys)?;
    fs::write(keys_file, json)?;
    println!("  {} Keys saved to {}", "✓".green(), keys_dir.display());
    Ok(())
}

/// Load keys from file.
pub fn load_keys() -> Result<Vec<KeyInfo>, Box<dyn std::error::Error>> {
    let keys_file = foc_localnet_keys().join("addresses.json");
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
        let bls_keys: Vec<_> = keys.iter().filter(|k| k.name.starts_with("BLS_") || k.name == "GLOBAL_FIL_FAUCET").collect();
        assert_eq!(bls_keys.len(), 3);
        
        // Check that BLS keys have t3 addresses
        for key in &bls_keys {
            assert!(key.filecoin_address.as_ref().unwrap().starts_with("t3"));
            assert!(key.eth_address.is_none());
            assert!(key.actor_id.is_none());
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
        
        let key = crate::crypto::derive_bls_key(&seed, 0).unwrap();
        
        // Check that we get a valid t3 address
        assert!(key.t3_address.starts_with("t3"));
        // t3 addresses can vary in length due to base32 encoding, but should be reasonable length
        assert!(key.t3_address.len() > 10 && key.t3_address.len() < 100);
        
        // Check that private key hex string is valid (BLS private key is 32 bytes = 64 hex chars)
        assert!(hex::decode(&key.private_key).is_ok());
        let pk_bytes = hex::decode(&key.private_key).unwrap();
        assert_eq!(pk_bytes.len(), 32);
        
        // Check that eth_address is empty for BLS
        assert_eq!(key.eth_address, "");
    }
}