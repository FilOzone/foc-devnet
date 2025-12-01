//! Constants for Lotus-Miner startup.
//!
//! This module contains all configuration constants used in the Lotus-Miner startup process.

pub const CONTAINER_NAME: &str = "foc-lotus-miner";
pub const IMAGE_NAME: &str = "foc-lotus-miner";

// Lotus-Miner ports
pub const LOTUS_MINER_PORTS: &[(u16, &str)] = &[(2345, "Lotus-Miner API")];

// Timing constants
pub const LOTUS_API_WAIT_SLEEP_SECS: u64 = 2;
pub const CONTAINER_INIT_WAIT_SECS: u64 = 15;
pub const MINER_API_CHECK_DELAY_SECS: u64 = 5;
pub const TIPSET_CHECK_DELAY_SECS: u64 = 10;
pub const PORT_WAIT_TIMEOUT_SECS: u64 = 45;
pub const CONTAINER_ID_DISPLAY_LENGTH: usize = 12;
