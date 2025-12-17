//! Constants used throughout the genesis preparation process.
//!
//! This module centralizes all configuration values related to genesis setup,
//! including sector parameters, network settings, and key configurations.

/// The size of sectors to pre-seal for the genesis miners.
///
/// This determines the storage capacity of the pre-sealed sectors that will be
/// included in the genesis block. Smaller sectors are used for testing and
/// development networks to reduce resource requirements.
pub const SECTOR_SIZE: &str = "2KiB";

/// The number of sectors to pre-seal for the genesis miners.
///
/// Multiple sectors allow each genesis miner to have initial storage capacity.
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
pub const NETWORK_NAME: &str = "2k";

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

/// The miner ID for the first genesis miner (Lotus miner).
///
/// This is the actor ID that will be used for the first miner in the genesis block.
/// The "t0" prefix indicates a testnet actor ID.
pub const LOTUS_MINER_ID: &str = "t01000";

/// The miner ID for the second genesis miner (Curio miner).
///
/// This is the actor ID that will be used for the second miner in the genesis block.
/// The "t0" prefix indicates a testnet actor ID.
pub const CURIO_MINER_ID: &str = "t01001";

/// The starting miner ID for PDP Service Providers.
///
/// PDP SP miners start from t01002 and increment sequentially (t01002, t01003, etc.)
/// This allows lotus-miner (t01000) and curio-miner (t01001) to have fixed IDs.
pub const PDP_SP_MINER_ID_START: u32 = 1002;

/// The number of active PDP Service Providers to initialize.
///
/// This determines how many PDP SP miners will have pre-sealed sectors created.
/// Keys for all MAX_PDP_SP_COUNT are generated, but only this many are activated.
pub const ACTIVE_PDP_SP_COUNT: usize = 1;
