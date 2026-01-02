//! Constants for Lotus-Miner startup.
//!
//! This module contains all configuration constants used in the Lotus-Miner startup process.

pub const IMAGE_NAME: &str = "foc-lotus-miner";

// Timing constants
pub const LOTUS_API_WAIT_SLEEP_SECS: u64 = 2;
pub const MINER_API_CHECK_DELAY_SECS: u64 = 5;
pub const PORT_WAIT_TIMEOUT_SECS: u64 = 45;
pub const CONTAINER_ID_DISPLAY_LENGTH: usize = 12;
