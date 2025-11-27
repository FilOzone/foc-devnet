//! Constants used throughout the genesis preparation process.
//!
//! This module centralizes all configuration values related to genesis setup,
//! including sector parameters, network settings, and key configurations.

/// The size of sectors to pre-seal for the genesis miner.
///
/// This determines the storage capacity of the pre-sealed sectors that will be
/// included in the genesis block. Smaller sectors are used for testing and
/// development networks to reduce resource requirements.
pub const SECTOR_SIZE: &str = "2KiB";

/// The number of sectors to pre-seal for the genesis miner.
///
/// Multiple sectors allow the genesis miner to have initial storage capacity.
/// This value should match the expected number of pre-seal files generated
/// by lotus-seed.
pub const NUM_SECTORS: u32 = 2;

/// The sector size parameter used when downloading Filecoin proof parameters.
///
/// This corresponds to the sector size that lotus will use for proof generation.
/// It must match the SECTOR_SIZE for consistency in the localnet setup.
pub const PROOF_PARAMS_SECTOR_SIZE: &str = "2048";

/// The name of the local Filecoin network being created.
///
/// This name is embedded in the genesis file and used to identify the network.
/// It should be unique to avoid conflicts with other networks.
pub const NETWORK_NAME: &str = "foc-localnet";

/// The filename of the genesis JSON file.
///
/// This file contains the complete genesis configuration including accounts,
/// miners, and network parameters. It will be created in the genesis directory.
pub const GENESIS_FILE: &str = "foc-localnet.json";

/// The threshold for multisig signer consensus.
///
/// This determines how many of the signer keys must approve transactions.
/// For a 2-of-2 multisig setup, this should be 2. For m-of-n, it would be m.
pub const SIGNERS_THRESHOLD: u32 = 2;

/// The number of BLS signer keys to generate.
///
/// These keys are used for the multisig wallet that controls network upgrades
/// and other privileged operations. Each signer gets an equal vote in consensus.
pub const NUM_SIGNER_KEYS: u32 = 2;

/// The number of additional pre-funded accounts to create beyond the signers.
///
/// These accounts are created with BLS keys and pre-funded with FIL tokens,
/// but do not have signing authority. They are useful for testing transactions
/// and smart contracts without affecting the signer accounts.
///
/// IMPORTANT: prefunded-1 is designated as GLOBAL_FIL_FAUCET and is used for:
/// - Transferring FIL to Ethereum addresses via f4 addresses
/// - Funding FOC (Filecoin Onchain Contracts) deployment operations
/// - General testing and development activities
///
/// To modify this value:
/// 1. Change this constant (e.g., to 3)
/// 2. Run `cargo run start` to regenerate the keys
/// 3. The keys will be available in ~/.foc-localnet/artifacts/docker/volumes/lotus-keys/prefunded-{1,2,3}/
/// 4. Import keys into lotus using: `lotus wallet import <keyinfo-file>`
///
/// Example: Set to 3 to create 3 additional pre-funded accounts.
pub const NUM_PREFUNDED_KEYS: u32 = 1;
const _: () = assert!(NUM_PREFUNDED_KEYS >= 1, "NUM_PREFUNDED_KEYS must be at least 1 for GLOBAL_FIL_FAUCET");
