//! Constants for endorsement operations.

/// Container name prefix for endorsement operations
pub const ENDORSEMENT_CONTAINER_PREFIX: &str = "foc-pdp-endorse";

/// Wait time after endorsement transaction (seconds)
pub const ENDORSEMENT_TX_WAIT_SECS: u64 = 10;

/// Maximum provider ID value (ProviderIdSet limitation)
pub const MAX_PROVIDER_ID: u64 = 0xFFFFFFFF;
