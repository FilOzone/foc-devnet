//! MockUSDFC token transfer operations.
//!
//! This module provides utilities for transferring MockUSDFC tokens between addresses.

use crate::commands::start::step::SetupContext;
use crate::docker::command_logger::run_and_log_command;
use crate::utils::retry::{retry_with_fixed_delay, DEFAULT_MAX_RETRIES, DEFAULT_RETRY_DELAY_SECS};
use ethers_core::types::U256;
use hex;
use std::error::Error;
use tracing::info;

/// Transfer MockUSDFC tokens from one address to another using cast
pub fn transfer_mock_usdfc(
    context: &SetupContext,
    from_private_key: &str,
    _from_eth_address: &str,
    to_eth_address: &str,
    amount: &str,
    token_address: &str,
    description: &str,
    nonce: Option<u64>,
    lotus_rpc_url: &str,
) -> Result<(), Box<dyn Error>> {
    info!("Transferring MockUSDFC tokens: {}...", description);

    let mut cast_cmd = format!(
        "cd /workspace && cast send {} \
         --private-key {} \
         --rpc-url {} \
         'transfer(address,uint256)' {} {} \
         --gas-limit 100000000",
        token_address, from_private_key, lotus_rpc_url, to_eth_address, amount
    );

    // Add nonce if provided
    if let Some(nonce_val) = nonce {
        cast_cmd.push_str(&format!(" --nonce {}", nonce_val));
    }

    // Debug output
    // println!("Executing command: {}", cast_cmd);

    let key = format!("usdfc_transfer_{}", description.replace(" ", "_"));
    let container_name = format!(
        "foc-{}-usdfc-transfer-{}",
        context.run_id(),
        description.replace(" ", "-").replace("→", "to")
    );
    let output = run_and_log_command(
        "docker",
        &[
            "run",
            "--name",
            &container_name,
            "--network",
            "host", // Use host network to access localhost:1234
            "-v",
            "/tmp:/workspace",
            crate::constants::BUILDER_DOCKER_IMAGE,
            "bash",
            "-c",
            &cast_cmd,
        ],
        context,
        &key,
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!(" Transfer failed");
        return Err(format!("Failed to transfer MockUSDFC: {}", stderr).into());
    }

    Ok(())
}

/// Check the MockUSDFC balance of an address
pub fn check_mock_usdfc_balance(
    context: &SetupContext,
    eth_address: &str,
    token_address: &str,
    lotus_rpc_url: &str,
) -> Result<U256, Box<dyn Error>> {
    // Retry balance check with fixed delay
    retry_with_fixed_delay(
        || {
            let key = format!("usdfc_balance_check_{}", eth_address);
            let container_name = format!(
                "foc-{}-usdfc-balance-check-{}",
                context.run_id(),
                &eth_address[..8]
            );
            let output = run_and_log_command(
                "docker",
                &[
                    "run",
                    "--rm",
                    "--name",
                    &container_name,
                    "--network",
                    "host",
                    "-v",
                    &format!(
                        "{}:/workspace",
                        crate::paths::project_root()?
                            .join("contracts/MockUSDFC")
                            .display()
                    ),
                    crate::constants::BUILDER_DOCKER_IMAGE,
                    "bash",
                    "-c",
                    &format!(
                        "cd /workspace && cast call {} \
                         --rpc-url {} \
                         'balanceOf(address)' {}",
                        token_address, lotus_rpc_url, eth_address
                    ),
                ],
                context,
                &key,
            )?;

            if !output.status.success() {
                return Err(format!(
                    "Failed to check balance for {}: {}",
                    eth_address,
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }

            let balance_hex = String::from_utf8_lossy(&output.stdout).trim().to_string();

            if balance_hex.is_empty() || balance_hex == "0x" {
                return Ok(U256::zero());
            }

            // Remove "0x" prefix if it exists
            let hex_str = balance_hex.strip_prefix("0x").unwrap_or(&balance_hex);

            // Decode hex to bytes
            let bytes = hex::decode(hex_str).map_err(|e| -> Box<dyn Error> {
                format!("Failed to decode hex string: {}: {}", hex_str, e).into()
            })?;

            // Convert bytes to U256
            let balance_u256 = U256::from_big_endian(&bytes);

            Ok(balance_u256)
        },
        DEFAULT_MAX_RETRIES,
        DEFAULT_RETRY_DELAY_SECS,
        &format!("MockUSDFC balance check for {}", eth_address),
    )
}
