//! Constants for endorsement operations.

/// Container name prefix for endorsement operations
pub const ENDORSEMENT_CONTAINER_PREFIX: &str = "foc-pdp-endorse";

/// Wait time after endorsement transaction (seconds)
pub const ENDORSEMENT_TX_WAIT_SECS: u64 = 10;

/// Gas limit for endorsement transactions on Filecoin FEVM
pub const ENDORSEMENT_GAS_LIMIT: &str = "10000000000";
