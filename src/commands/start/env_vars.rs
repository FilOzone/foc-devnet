//! Environment variable configuration for containers.
//!
//! This module provides utilities for building environment variable
//! arguments for Docker containers in the FOC localnet.

/// Build network parameter environment variables.
///
/// These are required for all Lotus, Lotus-Miner, and Curio containers.
///
/// # Returns
/// Vector of `-e KEY=VALUE` pairs for Docker run command
pub fn build_network_env_vars() -> Vec<String> {
    // vec![
    //     "-e".to_string(),
    //     format!("{}={}", ENV_FOC_LOCALNET_CHAIN_ID, LOCAL_NETWORK_CHAIN_ID),
    //     "-e".to_string(),
    //     format!("{}={}", ENV_FOC_LOCALNET_BLOCK_DELAY, FOC_LOCALNET_BLOCK_DELAY),
    //     "-e".to_string(),
    //     format!("{}={}", ENV_FOC_LOCALNET_PROPAGATION_DELAY, FOC_LOCALNET_PROPAGATION_DELAY),
    //     "-e".to_string(),
    //     format!("{}={}", ENV_FOC_LOCALNET_EQUIVOCATION_DELAY, FOC_LOCALNET_EQUIVOCATION_DELAY),
    // ]
    vec![]
}
