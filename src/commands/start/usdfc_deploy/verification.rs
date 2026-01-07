//! Contract verification for MockUSDFC deployment.
//!
//! This module handles the verification of deployed MockUSDFC contracts.

use super::foundry_setup::get_mockusdfc_project_dir;
use crate::commands::start::step::SetupContext;
use crate::docker::command_logger::run_and_log_command;
use crate::utils::retry::{retry_with_fixed_delay, DEFAULT_MAX_RETRIES, DEFAULT_RETRY_DELAY_SECS};
use std::error::Error;
use tracing::{info, warn};

/// Verify the deployed MockUSDFC contract
pub fn verify_mock_usdfc(
    context: &SetupContext,
    private_key: &str,
    contract_address: &str,
    lotus_rpc_url: &str,
    run_id: &str,
) -> Result<(), Box<dyn Error>> {
    info!("Verifying MockUSDFC contract functions...");

    // Get the contract directory from embedded assets
    let contract_dir = get_mockusdfc_project_dir(run_id)?;

    // Wait a bit for transaction confirmation
    info!("Waiting for transaction confirmation...");
    std::thread::sleep(std::time::Duration::from_secs(6));

    // Retry verification with fixed delay
    let verification_result = retry_with_fixed_delay(
        || {
            let verify_cmd = format!(
                "cd /workspace && \
                 forge script script/Verify.s.sol:VerifyMockUSDFC \
                 --rpc-url {} \
                 --private-key {} \
                 --sig 'run(address)' {} \
                 -vv",
                lotus_rpc_url, private_key, contract_address
            );

            let key = format!("usdfc_verify_{}", run_id);
            let output = run_and_log_command(
                "docker",
                &[
                    "run",
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
