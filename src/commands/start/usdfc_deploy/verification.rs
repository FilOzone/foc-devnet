//! Contract verification for MockUSDFC deployment.
//!
//! This module handles the verification of deployed MockUSDFC contracts.

use crate::commands::start::step::SetupContext;
use crate::docker::command_logger::run_and_log_command;
use crate::utils::retry::{retry_with_fixed_delay, DEFAULT_MAX_RETRIES, DEFAULT_RETRY_DELAY_SECS};
use std::error::Error;
use std::path::Path;
use tracing::{info, warn};

/// Time to wait for transaction confirmation before verification (in seconds)
const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 6;

/// Verify the deployed MockUSDFC contract
pub fn verify_mock_usdfc(
    context: &SetupContext,
    private_key: &str,
    contract_address: &str,
    lotus_rpc_url: &str,
    run_id: &str,
    contract_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    info!("Verifying MockUSDFC contract functions...");

    // Wait a bit for transaction confirmation
    info!("Waiting for transaction confirmation...");
    std::thread::sleep(std::time::Duration::from_secs(
        TRANSACTION_CONFIRMATION_WAIT_SECS,
    ));

    // Retry verification with fixed delay
    let verification_result = retry_with_fixed_delay(
        || {
            let verify_cmd = format!(
                "set -euo pipefail && \
                 DEPLOYER=$(cast wallet address --private-key {}) && \
                 NAME=$(cast call --rpc-url {} {} 'name()(string)') && \
                 SYMBOL=$(cast call --rpc-url {} {} 'symbol()(string)') && \
                 DECIMALS=$(cast call --rpc-url {} {} 'decimals()(uint8)') && \
                 TOTAL_SUPPLY=$(cast call --rpc-url {} {} 'totalSupply()(uint256)') && \
                 BALANCE=$(cast call --rpc-url {} {} 'balanceOf(address)(uint256)' $DEPLOYER) && \
                 echo \"Name: $NAME\" && \
                 echo \"Symbol: $SYMBOL\" && \
                 echo \"Decimals: $DECIMALS\" && \
                 echo \"Total supply: $TOTAL_SUPPLY\" && \
                 echo \"Deployer balance: $BALANCE\" && \
                 TOTAL_SUPPLY_VALUE=$(echo \"$TOTAL_SUPPLY\" | awk '{{print $1}}') && \
                 BALANCE_VALUE=$(echo \"$BALANCE\" | awk '{{print $1}}') && \
                 ([ \"$NAME\" = 'Mock USDC' ] || [ \"$NAME\" = '\"Mock USDC\"' ]) && \
                 ([ \"$SYMBOL\" = 'USDFC' ] || [ \"$SYMBOL\" = '\"USDFC\"' ]) && \
                 [ \"$DECIMALS\" = '18' ] && \
                 [ \"$TOTAL_SUPPLY_VALUE\" = '{}' ] && \
                 [ \"$BALANCE_VALUE\" = '{}' ]",
                private_key,
                lotus_rpc_url,
                contract_address,
                lotus_rpc_url,
                contract_address,
                lotus_rpc_url,
                contract_address,
                lotus_rpc_url,
                contract_address,
                lotus_rpc_url,
                contract_address,
                crate::constants::MOCK_USDFC_INITIAL_SUPPLY,
                crate::constants::MOCK_USDFC_INITIAL_SUPPLY
            );

            let key = format!("usdfc_verify_{}", run_id);
            let container_name = format!("foc-{}-usdfc-verify", run_id);
            let output = run_and_log_command(
                "docker",
                &[
                    "run",
                    "--rm",
                    "--name",
                    &container_name,
                    "-u",
                    "foc-user",
                    "--network",
                    "host",
                    "-v",
                    &format!("{}:/workspace", contract_dir.display()),
                    crate::constants::BUILDER_DOCKER_IMAGE,
                    "bash",
                    "-c",
                    &verify_cmd,
                ],
                context,
                &key,
            )?;

            let _stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !output.status.success() {
                return Err(format!(
                    "Verification failed: {}",
                    if !stderr.is_empty() {
                        stderr.to_string()
                    } else {
                        "Unknown error".to_string()
                    }
                )
                .into());
            }

            Ok(())
        },
        DEFAULT_MAX_RETRIES,
        DEFAULT_RETRY_DELAY_SECS,
        "MockUSDFC contract verification",
    );

    match verification_result {
        Ok(_) => {
            info!("✓ All contract functions verified");
        }
        Err(e) => {
            warn!("Contract verification failed after retries: {}", e);
            warn!("Continuing despite verification warning");
        }
    }

    Ok(())
}
