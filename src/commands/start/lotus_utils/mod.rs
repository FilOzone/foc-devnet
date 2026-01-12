//! Lotus utilities for starting containers.
//!
//! This module provides shared utilities for working with Lotus daemon.

use std::error::Error;
use std::fs;

use super::step::SetupContext;
use crate::paths::foc_devnet_docker_volumes_run_specific;

/// Read the Lotus API token from the lotus-data directory.
///
/// The token is written by the Lotus daemon after it starts and is required
/// for authentication when connecting to the Lotus API.
///
/// # Returns
/// The token string if found, or an error if the token file doesn't exist or can't be read.
pub fn read_lotus_token(run_id: &str) -> Result<String, Box<dyn Error>> {
    let token_path = foc_devnet_docker_volumes_run_specific(run_id)
        .join("lotus-data")
        .join("token");

    if !token_path.exists() {
        return Err(format!(
            "Lotus token file not found at {}. Ensure Lotus daemon is running.",
            token_path.display()
        )
        .into());
    }

    let token = fs::read_to_string(&token_path)?.trim().to_string();

    if token.is_empty() {
        return Err("Lotus token file is empty".into());
    }

    Ok(token)
}

/// Build the FULLNODE_API_INFO environment variable value.
///
/// Format: `<token>:/dns4/<lotus_container_name>/tcp/1234/http`
///
/// # Arguments
/// * `token` - The Lotus API token
/// * `lotus_container_name` - The name of the Lotus container on the Docker network
pub fn build_fullnode_api_info(token: &str, lotus_container_name: &str) -> String {
    format!("{}:/dns4/{}/tcp/1234/http", token, lotus_container_name)
}

/// Get the Lotus RPC URL from context with dynamically allocated port.
///
/// This function retrieves the Lotus API port that was dynamically allocated
/// during startup and builds the complete RPC URL for contract deployment
/// and other interactions.
///
/// # Arguments
///
/// * `context` - The SetupContext containing the allocated lotus_api_port
///
/// # Returns
///
/// The full RPC URL (e.g., "http://localhost:5700/rpc/v1")
///
/// # Errors
///
/// Returns an error if the lotus_api_port is not found in context or cannot be parsed
pub fn get_lotus_rpc_url(context: &SetupContext) -> Result<String, Box<dyn Error>> {
    let lotus_api_port: u16 = context
        .get("lotus_api_port")
        .ok_or("Lotus API port not found in context")?
        .parse()?;
    Ok(format!("http://localhost:{}/rpc/v1", lotus_api_port))
}
