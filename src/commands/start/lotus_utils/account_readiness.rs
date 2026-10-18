//! Read-only checks for account readiness in the Lotus message pool.

use super::super::step::SetupContext;
use crate::docker::command_logger::run_and_log_command_strings;
use crate::utils::retry::{retry_with_fixed_delay, DEFAULT_MAX_RETRIES, DEFAULT_RETRY_DELAY_SECS};
use std::error::Error;
use tracing::info;

/// Wait until the message pool can resolve an account's nonce.
///
/// A funded balance at the chain head does not mean the message pool has caught up.
/// `--block pending` queries the pool's state view, also used when broadcasting;
/// other block parameters can return zero for a missing actor and hide this race.
/// This checks actor visibility, not every condition for transaction acceptance.
pub fn wait_for_account_nonce(
    eth_address: &str,
    lotus_rpc_url: &str,
    context: &SetupContext,
) -> Result<(), Box<dyn Error>> {
    let run_id = context.run_id();
    let mut attempt = 0;

    retry_with_fixed_delay(
        || {
            attempt += 1;
            let args: Vec<String> = vec![
                "run".to_string(),
                "--rm".to_string(),
                "--name".to_string(),
                format!("foc-{}-nonce-{}-{}", run_id, eth_address, attempt),
                "-u".to_string(),
                "foc-user".to_string(),
                "--network".to_string(),
                "host".to_string(),
                crate::constants::BUILDER_DOCKER_IMAGE.to_string(),
                "bash".to_string(),
                "-c".to_string(),
                format!(
                    "cast nonce {} --block pending --rpc-url {}",
                    eth_address, lotus_rpc_url
                ),
            ];

            let key = format!("account_nonce_{}_{}_{}", run_id, eth_address, attempt);
            let output = run_and_log_command_strings("docker", &args, context, &key)?;

            if output.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Pending nonce lookup failed for {}: {}",
                    eth_address,
                    String::from_utf8_lossy(&output.stderr).trim()
                )
                .into())
            }
        },
        DEFAULT_MAX_RETRIES,
        DEFAULT_RETRY_DELAY_SECS,
        &format!("Account nonce lookup for {}", eth_address),
    )?;

    info!("✓ Message pool resolves the nonce for {}", eth_address);
    Ok(())
}
