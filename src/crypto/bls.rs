//! BLS cryptographic key generation and address computation utilities.
//!
//! This module provides functions for generating BLS keys and computing
//! Filecoin BLS addresses.

use bls_signatures::{PrivateKey, PublicKey, Serialize};
use sha3::{Digest, Keccak256};

use crate::crypto::DerivedKey;

/// Derive a deterministic Filecoin BLS key from seed
/// using HD wallet derivation.
///
/// The derivation path is:
/// `m/44'/461'/0'/0/<acc_str_hash>`
pub fn derive_bls_key(
    seed: &[u8; 64],
    acc_str: &str,
) -> Result<DerivedKey, Box<dyn std::error::Error>> {
    let acc_str = acc_str.to_string();
    // Hash the account string to get a unique index
    let mut hasher = Keccak256::new();
    hasher.update(acc_str.as_bytes());
    let hash = hasher.finalize();
    let path = format!("m/44'/461'/0'/0/{}", hex::encode(hash));

    let private_key_bytes = derive_private_key_bytes(seed, &path)?;

    // Create BLS private key using the proper key generation method
    // PrivateKey::new() uses HKDF to derive the actual BLS key from the seed
    let bls_private_key = PrivateKey::new(private_key_bytes);
    let bls_public_key = bls_private_key.public_key();

    // Serialize the actual BLS private key (not the input seed)
    let bls_private_key_bytes = bls_private_key.as_bytes();

    // Compute BLS address (f3/t3)
    let bls_address = compute_bls_address(&bls_public_key)?;

    Ok(DerivedKey {
        private_key: hex::encode(bls_private_key_bytes),
        native_address: bls_address,
        eth_address: None,
    })
}

/// Derive 32 bytes of key material from seed and derivation path.
fn derive_private_key_bytes(
    seed: &[u8; 64],
    path: &str,
) -> Result<[u8; 32], Box<dyn std::error::Error>> {
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
    let address =
        base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &address_bytes).to_lowercase();

    // Step 6: Construct final address with network and protocol prefix
    // Format: "t" + "3" + base32_encoded_data
    // "t" indicates testnet (use "f" for mainnet)
    // "3" indicates BLS protocol
    // Result: "t3" + 84 chars = 86 total characters
    Ok(format!("t3{}", address))
}
