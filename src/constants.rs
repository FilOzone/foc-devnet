//! Constants used throughout the foc-devnet codebase.
//!
//! This module centralizes all magic numbers, container names, port numbers,
//! and other constants to avoid scattering them throughout the codebase.

/// Docker image names
pub const LOTUS_DOCKER_IMAGE: &str = "foc-lotus";
pub const LOTUS_MINER_DOCKER_IMAGE: &str = "foc-lotus-miner";
pub const BUILDER_DOCKER_IMAGE: &str = "foc-builder";
pub const CURIO_DOCKER_IMAGE: &str = "foc-curio";
pub const PORTAINER_DOCKER_IMAGE: &str = "foc-portainer";

/// Stock database images, pulled on demand by the database step (not built).
/// Curio's HarmonyDB speaks Postgres and its IndexStore speaks Cassandra/CQL;
/// Scylla serves the latter.
pub const POSTGRES_DOCKER_IMAGE: &str = "postgres:18";
pub const SCYLLA_DOCKER_IMAGE: &str = "scylladb/scylla:2026.1";

/// Database container ports (inside the container) and shared credentials.
/// The database step provisions Postgres with these and the Curio env wiring
/// (start/curio/db_setup.rs) connects with them; they must agree.
pub const POSTGRES_CONTAINER_PORT: u16 = 5432;
pub const SCYLLA_CQL_CONTAINER_PORT: u16 = 9042;
pub const DB_USER: &str = "curio";
pub const DB_PASSWORD: &str = "curio";
pub const DB_NAME: &str = "curio";

/// Required binaries for cluster startup
pub const REQUIRED_BINARIES: &[&str] = &[
    "lotus",
    "lotus-miner",
    "lotus-shed",
    "lotus-seed",
    "curio",
    "pdptool",
    "sptool",
];

/// Images foc-devnet builds itself. Used to scope destructive cleanup to our own
/// images; stock images (postgres, scylla) are shared and must not be swept.
pub const FOC_BUILT_IMAGES: &[&str] = &[
    LOTUS_DOCKER_IMAGE,
    LOTUS_MINER_DOCKER_IMAGE,
    BUILDER_DOCKER_IMAGE,
    CURIO_DOCKER_IMAGE,
];

/// foc-built images that must exist before a cluster can start. The stock
/// database images (postgres/scylla) are not listed: the database step pulls
/// them on demand.
pub const REQUIRED_DOCKER_IMAGES: &[&str] = &[
    LOTUS_DOCKER_IMAGE,
    LOTUS_MINER_DOCKER_IMAGE,
    BUILDER_DOCKER_IMAGE,
    CURIO_DOCKER_IMAGE,
];

/// Check whether a Docker image identifier (optionally tagged, e.g. "foc-lotus:latest")
/// is one foc-devnet builds. Used to scope destructive cleanup to our own images
/// and avoid sweeping up unrelated images that share a "foc-" prefix (e.g.
/// foc-observer-*) or shared stock images (postgres, scylla).
pub fn is_foc_devnet_image(image: &str) -> bool {
    let repo = image.split(':').next().unwrap_or(image);
    FOC_BUILT_IMAGES.contains(&repo)
}

/// Docker container names (base - will be prefixed with foc-c-<RUN_ID>- in practice)
pub const LOTUS_CONTAINER: &str = "foc-lotus";
pub const LOTUS_MINER_CONTAINER: &str = "foc-lotus-miner";
pub const BUILDER_CONTAINER: &str = "foc-builder";
pub const CURIO_CONTAINER: &str = "foc-curio";
pub const PORTAINER_CONTAINER: &str = "foc-portainer";

/// Port numbers
pub const LOTUS_RPC_PORT: u16 = 1234;
pub const LOTUS_MINER_API_PORT: u16 = 2345;
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

/// PDP Service Provider configuration
pub const MAX_PDP_SP_COUNT: usize = 5;

/// Number of user test accounts (USER_1, USER_2, USER_3)
pub const USER_ACCOUNT_COUNT: usize = 3;

/// Service configuration
pub const SERVICE_NAME: &str = "FOC DevNet Warm Storage";
pub const SERVICE_DESCRIPTION: &str = "Warm storage service for FOC local development network";

/// Token parameters
pub const MOCK_USDFC_INITIAL_SUPPLY: &str = "1000000000000000000000000"; // 1,000,000 tokens
pub const MOCK_USDFC_DECIMALS: u8 = 18;
pub const MOCK_USDFC_SYMBOL: &str = "USDFC";
pub const MOCK_USDFC_NAME: &str = "Mock USDFC";

/// Network configuration
pub const LOCAL_NETWORK_CHAIN_ID: u64 = 31415926; // Local network chain ID

/// devnet network parameters (for Lotus, Lotus-Miner, and Curio)
pub const FOC_DEVNET_BLOCK_DELAY: u64 = 4; // Block delay in seconds
pub const FOC_DEVNET_PROPAGATION_DELAY: u64 = 2; // Propagation delay in seconds
pub const FOC_DEVNET_EQUIVOCATION_DELAY: u64 = 0; // Equivocation delay in seconds

/// Simple service contract address (zero address)
pub const FOC_DEVNET_CONTRACT_SIMPLE: &str = "0x0000000000000000000000000000000000000000";

/// Environment variable names
pub const ENV_FOC_DEVNET_CHAIN_ID: &str = "FOC_DEVNET_CHAIN_ID";
pub const ENV_FOC_DEVNET_BLOCK_DELAY: &str = "FOC_DEVNET_BLOCK_DELAY";
pub const ENV_FOC_DEVNET_PROPAGATION_DELAY: &str = "FOC_DEVNET_PROPAGATION_DELAY";
pub const ENV_FOC_DEVNET_EQUIVOCATION_DELAY: &str = "FOC_DEVNET_EQUIVOCATION_DELAY";
pub const ENV_FOC_DEVNET_CONTRACT_PAY: &str = "FOC_CONTRACT_PAY";
pub const ENV_FOC_DEVNET_CONTRACT_FWSS: &str = "FOC_CONTRACT_FWSS";
pub const ENV_FOC_DEVNET_CONTRACT_MULTICALL: &str = "FOC_CONTRACT_MULTICALL";
pub const ENV_FOC_DEVNET_CONTRACT_SIMPLE: &str = "FOC_CONTRACT_SIMPLE";
pub const ENV_FOC_DEVNET_CONTRACT_USDFC: &str = "FOC_CONTRACT_USDFC";

/// Curio logging configuration
pub const CURIO_LOG_LEVEL: &str = "GOLOG_LOG_LEVEL=pdp=debug";

/// File paths within containers
pub const LOTUS_BINARY_PATH: &str = "/usr/local/bin/lotus-bins/lotus";
pub const LOTUS_MINER_BINARY_PATH: &str = "/usr/local/bin/lotus-bins/lotus-miner";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_foc_devnet_image_accepts_known_images() {
        for image in FOC_BUILT_IMAGES {
            assert!(is_foc_devnet_image(image), "{} should match", image);
            assert!(
                is_foc_devnet_image(&format!("{}:latest", image)),
                "{}:latest should match",
                image
            );
        }
    }

    #[test]
    fn test_is_foc_devnet_image_rejects_unrelated() {
        assert!(!is_foc_devnet_image("foc-observer-foc-observer"));
        assert!(!is_foc_devnet_image("foc-observer-foc-observer:latest"));
        assert!(!is_foc_devnet_image("foc-observer-ponder-mainnet"));
        assert!(!is_foc_devnet_image("portainer/portainer-ce:latest"));
        assert!(!is_foc_devnet_image("foc-portainer"));
        assert!(!is_foc_devnet_image("foc-yugabyte"));
        assert!(!is_foc_devnet_image("nginx"));
        assert!(!is_foc_devnet_image("postgres:18"));
        assert!(!is_foc_devnet_image("scylladb/scylla:latest"));
        assert!(!is_foc_devnet_image(""));
    }
}
