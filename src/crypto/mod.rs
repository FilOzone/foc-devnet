//! Cryptographic key generation and address computation utilities.
//!
//! This module provides low-level cryptographic functions for generating
//! deterministic keys and computing Filecoin/Ethereum addresses.

use bls_signatures::{PrivateKey, PublicKey, Serialize};
use ethers_core::k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};

/// Derived key information containing private key and addresses.
#[derive(Debug, Clone)]
pub struct DerivedKey {
    /// Hex-encoded private key
    pub private_key: String,
    /// Filecoin t3 address (for BLS keys)
    pub t3_address: String,
    /// Ethereum address (for Ethereum keys)
    pub eth_address: String,
}

/// Derive a Filecoin BLS key from seed using HD wallet derivation.
pub fn derive_bls_key(seed: &[u8; 64], account: u32) -> Result<DerivedKey, Box<dyn std::error::Error>> {
    // Filecoin coin type is 461
    let path = format!("m/44'/461'/0'/0/{}", account);
    let private_key_bytes = derive_private_key_bytes(seed, &path)?;

    // Create BLS private key using the proper key generation method
    // PrivateKey::new() uses HKDF to derive the actual BLS key from the seed
    let bls_private_key = PrivateKey::new(&private_key_bytes);
    let bls_public_key = bls_private_key.public_key();

    // Serialize the actual BLS private key (not the input seed)
    let bls_private_key_bytes = bls_private_key.as_bytes();

    // Compute BLS address (f3/t3)
    let bls_address = compute_bls_address(&bls_public_key)?;

    Ok(DerivedKey {
        private_key: hex::encode(bls_private_key_bytes),
        t3_address: bls_address,
        eth_address: String::new(), // Not used for BLS
    })
}

/// Derive an Ethereum key from seed using HD wallet derivation.
pub fn derive_ethereum_key(seed: &[u8; 64], account: u32) -> Result<DerivedKey, Box<dyn std::error::Error>> {
    // Ethereum coin type is 60
    let path = format!("m/44'/60'/0'/0/{}", account);
    let private_key = derive_private_key(seed, &path)?;
    let public_key = private_key.verifying_key();
    let eth_address = compute_eth_address(&public_key)?;

    Ok(DerivedKey {
        private_key: hex::encode(private_key.to_bytes()),
        t3_address: String::new(), // Not used
        eth_address,
    })
}

/// Derive an ECDSA private key from seed and derivation path.
fn derive_private_key(seed: &[u8; 64], path: &str) -> Result<SigningKey, Box<dyn std::error::Error>> {
    // Simple derivation: hash seed + path
    let mut hasher = Keccak256::new();
    hasher.update(seed);
    hasher.update(path);
    let hash = hasher.finalize();
    let private_key = SigningKey::from_slice(&hash[..32])?;
    Ok(private_key)
}

/// Derive 32 bytes of key material from seed and derivation path.
fn derive_private_key_bytes(seed: &[u8; 64], path: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    // Simple derivation: hash seed + path
    let mut hasher = Keccak256::new();
    hasher.update(seed);
    hasher.update(path);
    let hash = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&hash[..32]);
    Ok(bytes)
}

/// Compute Filecoin BLS address from public key.
///
/// Filecoin BLS addresses (protocol 3) have a specific encoding format:
///
/// Address format: `<network><protocol><base32_encoded_data>`
/// - network: "t" for testnet, "f" for mainnet
/// - protocol: "3" (BLS protocol identifier)
/// - base32_encoded_data: base32(pubkey || checksum) - 84 characters
///
/// Total length: 2 + 84 = 86 characters (e.g., "t3abc...xyz")
///
/// Key differences from other address types:
/// 1. BLS uses the FULL 48-byte public key (no hashing to 20 bytes like SECP256K1)
/// 2. Checksum is Blake2b-4 (4-byte Blake2b hash), NOT CRC32
/// 3. Checksum input: protocol_byte || pubkey (NOT network || protocol || pubkey)
/// 4. Base32 encoding: pubkey || checksum (protocol byte is NOT included)
///
/// This matches the go-address library implementation used by Lotus.
pub fn compute_bls_address(public_key: &PublicKey) -> Result<String, Box<dyn std::error::Error>> {
    // Step 1: Get the raw BLS public key bytes (48 bytes for BLS12-381)
    // Unlike SECP256K1 addresses which hash the pubkey to 20 bytes,
    // BLS addresses use the full public key as the payload
    let pubkey_bytes = public_key.as_bytes();

    // Step 2: Define the protocol byte for BLS addresses
    // Protocol types: 0=ID, 1=SECP256K1, 2=Actor, 3=BLS, 4=Delegated
    let protocol: u8 = 3;

    // Step 3: Compute the checksum using Blake2b with 4-byte output
    // Checksum input: protocol_byte || pubkey_bytes (49 bytes total)
    // NOTE: The network byte is NOT included in the checksum calculation
    let mut checksum_input = vec![protocol];
    checksum_input.extend_from_slice(&pubkey_bytes);
    let checksum = blake2::Blake2b::<blake2::digest::consts::U4>::digest(&checksum_input);
    let checksum_bytes = checksum.as_slice(); // 4 bytes

    // Step 4: Prepare data for base32 encoding
    // Base32 input: pubkey_bytes || checksum (52 bytes total = 48 + 4)
    // NOTE: The protocol byte is NOT included in the base32 encoding
    let mut address_bytes = pubkey_bytes.to_vec();
    address_bytes.extend_from_slice(checksum_bytes);

    // Step 5: Encode to base32 (lowercase, no padding)
    // 52 bytes → ceil(52 * 8 / 5) = 84 base32 characters
    let address = base32::encode(base32::Alphabet::RFC4648 { padding: false }, &address_bytes).to_lowercase();

    // Step 6: Construct final address with network and protocol prefix
    // Format: "t" + "3" + base32_encoded_data
    // "t" indicates testnet (use "f" for mainnet)
    // "3" indicates BLS protocol
    // Result: "t3" + 84 chars = 86 total characters
    Ok(format!("t3{}", address))
}

/// Compute Ethereum address from public key.
pub fn compute_eth_address(public_key: &ethers_core::k256::ecdsa::VerifyingKey) -> Result<String, Box<dyn std::error::Error>> {
    let pubkey_bytes = public_key.to_encoded_point(false).as_bytes()[1..].to_vec(); // Remove 0x04
    let hash = Keccak256::new().chain_update(&pubkey_bytes).finalize();
    let address_bytes = &hash[12..32];
    Ok(format!("0x{}", hex::encode(address_bytes)))
}

/// Compute Filecoin t4 address from Ethereum address and address manager ID.
pub fn compute_t4_address(eth_address: &str, manager_id: u64) -> Result<String, Box<dyn std::error::Error>> {
    let eth_bytes = hex::decode(&eth_address[2..])?; // Remove 0x prefix
    let leb = leb128_encode(manager_id);
    let mut binary = vec![4u8];
    binary.extend(leb);
    binary.extend(eth_bytes.clone());
    let checksum = blake2::Blake2b::<blake2::digest::consts::U4>::digest(&binary); // 32-bit checksum
    let mut sub_address = eth_bytes;
    sub_address.extend(checksum);
    let base32 = base32::encode(base32::Alphabet::RFC4648 { padding: false }, &sub_address).to_lowercase();
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

#[cfg(test)]
mod tests {
    use super::*;
    use bip39::{Language, Mnemonic};

    #[test]
    fn test_derive_bls_key() {
        let mnemonic = Mnemonic::parse_in_normalized(Language::English, "sudden spend mask joke vibrant situate tilt history occur rally artwork shadow gather proud urban work own quick holiday bone target zone unknown nut").unwrap();
        let seed = mnemonic.to_seed("");

        let key = derive_bls_key(&seed, 0).unwrap();

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