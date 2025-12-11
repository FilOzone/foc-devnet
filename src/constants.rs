//! Constants used throughout the foc-localnet codebase.
//!
//! This module centralizes all magic numbers, container names, port numbers,
//! and other constants to avoid scattering them throughout the codebase.

/// Container names (base - will be prefixed with foc-<RUN_ID>- in practice)
pub const LOTUS_CONTAINER: &str = "foc-lotus";
pub const LOTUS_MINER_CONTAINER: &str = "foc-lotus-miner";
pub const BUILDER_CONTAINER: &str = "foc-builder";
pub const YUGABYTE_CONTAINER: &str = "foc-yugabyte";
pub const CURIO_CONTAINER: &str = "foc-curio";
pub const PORTAINER_CONTAINER: &str = "foc-portainer";

/// Port numbers
pub const LOTUS_RPC_PORT: u16 = 1234;
pub const LOTUS_MINER_API_PORT: u16 = 2345;
pub const YUGABYTE_PORT: u16 = 5433;
pub const PORTAINER_PORT: u16 = 9009;

/// Sleep durations (in seconds)
pub const CONTAINER_INIT_WAIT_SECS: u64 = 5;
pub const DAEMON_INIT_WAIT_SECS: u64 = 10;
pub const API_FILE_TIMEOUT_SECS: u64 = 180; // 3 minutes
pub const CURIO_START_WAIT_SECS: u64 = 30;
pub const API_CHECK_DELAY_SECS: u64 = 5;
pub const LOTUS_API_WAIT_SLEEP_SECS: u64 = 2;
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 15;

/// Sleep durations (in milliseconds)
pub const PORT_CHECK_INTERVAL_MS: u64 = 100;
pub const PORT_CHECK_TIMEOUT_MS: u64 = 5000;

/// Service configuration
pub const SERVICE_NAME: &str = "FOC LocalNet Warm Storage";
pub const SERVICE_DESCRIPTION: &str = "Warm storage service for FOC local development network";

/// Token parameters
pub const MOCK_USDFC_INITIAL_SUPPLY: &str = "1000000000000000000000000"; // 1,000,000 tokens
pub const MOCK_USDFC_DECIMALS: u8 = 18;
pub const MOCK_USDFC_SYMBOL: &str = "USDFC";
pub const MOCK_USDFC_NAME: &str = "Mock USDFC";

/// Docker image names
pub const LOTUS_IMAGE: &str = "foc-lotus";
pub const LOTUS_MINER_IMAGE: &str = "foc-lotus-miner";
pub const BUILDER_IMAGE: &str = "foc-builder";
pub const YUGABYTE_IMAGE: &str = "foc-yugabyte";
pub const CURIO_IMAGE: &str = "foc-curio";

/// Network configuration
pub const LOCAL_NETWORK_CHAIN_ID: u64 = 1414; // Local network chain ID

/// Localnet network parameters (for Lotus, Lotus-Miner, and Curio)
pub const FOC_LOCALNET_BLOCK_DELAY: u64 = 4; // Block delay in seconds
pub const FOC_LOCALNET_PROPAGATION_DELAY: u64 = 2; // Propagation delay in seconds
pub const FOC_LOCALNET_EQUIVOCATION_DELAY: u64 = 0; // Equivocation delay in seconds

/// Simple service contract address (zero address)
pub const FOC_LOCALNET_CONTRACT_SIMPLE: &str = "0x0000000000000000000000000000000000000000";

/// Environment variable names
pub const ENV_FOC_LOCALNET_CHAIN_ID: &str = "FOC_LOCALNET_CHAIN_ID";
pub const ENV_FOC_LOCALNET_BLOCK_DELAY: &str = "FOC_LOCALNET_BLOCK_DELAY";
pub const ENV_FOC_LOCALNET_PROPAGATION_DELAY: &str = "FOC_LOCALNET_PROPAGATION_DELAY";
pub const ENV_FOC_LOCALNET_EQUIVOCATION_DELAY: &str = "FOC_LOCALNET_EQUIVOCATION_DELAY";
pub const ENV_FOC_LOCALNET_CONTRACT_PAY: &str = "FOC_CONTRACT_PAY";
pub const ENV_FOC_LOCALNET_CONTRACT_FWSS: &str = "FOC_CONTRACT_FWSS";
pub const ENV_FOC_LOCALNET_CONTRACT_MULTICALL: &str = "FOC_CONTRACT_MULTICALL";
pub const ENV_FOC_LOCALNET_CONTRACT_SIMPLE: &str = "FOC_CONTRACT_SIMPLE";
pub const ENV_FOC_LOCALNET_CONTRACT_USDFC: &str = "FOC_CONTRACT_USDFC";

/// File paths within containers
pub const LOTUS_BINARY_PATH: &str = "/usr/local/bin/lotus-bins/lotus";
pub const LOTUS_MINER_BINARY_PATH: &str = "/usr/local/bin/lotus-bins/lotus-miner";
