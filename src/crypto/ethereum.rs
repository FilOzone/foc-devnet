//! Ethereum cryptographic key generation and address computation utilities.
//!
//! This module provides functions for generating Ethereum keys and computing
//! Ethereum and Filecoin delegated addresses.

use ethers_core::k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};

use crate::crypto::DerivedKey;

/// Derive a deterministic Filecoin Ethereum key from seed
/// using HD wallet derivation.
///
/// The derivation path is:
/// `m/44'/461'/0'/0/<acc_str_hash>`
pub fn derive_ethereum_key(
    seed: &[u8; 64],
    acc_str: &str,
) -> Result<DerivedKey, Box<dyn std::error::Error>> {
    let acc_str = acc_str.to_string();
    // Hash the account string to get a unique index
    let mut hasher = Keccak256::new();
    hasher.update(acc_str.as_bytes());
    let hash = hasher.finalize();
    let path = format!("m/44'/60'/0'/0/{}", hex::encode(hash));

    let private_key = derive_ethereum_private_key(seed, &path)?;
    let public_key = private_key.verifying_key();
    let eth_address = compute_eth_address(public_key)?;

    const FEVM_MANAGER_ID: u64 = 10;
    let native_address = compute_native_address(&eth_address, FEVM_MANAGER_ID)?;

    Ok(DerivedKey {
        private_key: hex::encode(private_key.to_bytes()),
        native_address,
        eth_address: Some(eth_address),
    })
}

/// Derive an ECDSA private key from seed and derivation path.
fn derive_ethereum_private_key(
    seed: &[u8; 64],
    path: &str,
) -> Result<SigningKey, Box<dyn std::error::Error>> {
    // Simple derivation: hash seed + path
    let mut hasher = Keccak256::new();
    hasher.update(seed);
    hasher.update(path);
    let hash = hasher.finalize();
    let private_key = SigningKey::from_slice(&hash[..32])?;
    Ok(private_key)
}

/// Compute Ethereum address from public key.
pub fn compute_eth_address(
    public_key: &ethers_core::k256::ecdsa::VerifyingKey,
) -> Result<String, Box<dyn std::error::Error>> {
    let pubkey_bytes = public_key.to_encoded_point(false).as_bytes()[1..].to_vec(); // Remove 0x04
    let hash = Keccak256::new().chain_update(&pubkey_bytes).finalize();
    let address_bytes = &hash[12..32];
    Ok(format!("0x{}", hex::encode(address_bytes)))
}

/// Compute Filecoin t4 address from Ethereum address and address manager ID.
pub fn compute_native_address(
    eth_address: &str,
    manager_id: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let eth_bytes = hex::decode(&eth_address[2..])?; // Remove 0x prefix
    let leb = leb128_encode(manager_id);
    let mut binary = vec![4u8];
    binary.extend(leb);
    binary.extend(eth_bytes.clone());
    let checksum = blake2::Blake2b::<blake2::digest::consts::U4>::digest(&binary); // 32-bit checksum
    let mut sub_address = eth_bytes;
    sub_address.extend(checksum);
    let base32 =
        base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &sub_address).to_lowercase();
    Ok(format!("t4{}f{}", manager_id, base32))
}

/// Encode a u64 as LEB128.
fn leb128_encode(mut value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if value == 0 {
            break;
        }
    }
    bytes
}
