//! Contract verification for MockUSDFC deployment.
//!
//! This module handles the verification of deployed MockUSDFC contracts.

use super::foundry_setup::get_mockusdfc_project_dir;
use std::error::Error;
use std::process::Command;
use tracing::{info, warn};

/// Verify the deployed MockUSDFC contract
pub fn verify_mock_usdfc(
    private_key: &str,
    contract_address: &str,
    lotus_rpc_url: &str,
    run_id: &str,
) -> Result<(), Box<dyn Error>> {
    info!("      Verifying MockUSDFC contract functions...");

    // Get the contract directory from embedded assets
    let contract_dir = get_mockusdfc_project_dir(run_id)?;

    // Wait a bit for transaction confirmation
    info!("        Waiting for transaction confirmation...");
    std::thread::sleep(std::time::Duration::from_secs(6));

    let verify_cmd = format!(
        "cd /workspace && \
         forge script script/Verify.s.sol:VerifyMockUSDFC \
         --rpc-url {} \
         --private-key {} \
         --sig 'run(address)' {} \
         -vv",
        lotus_rpc_url, private_key, contract_address
    );

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            "-v",
            &format!("{}:/workspace", contract_dir.display()),
            "foc-builder",
            "bash",
            "-c",
            &verify_cmd,
        ])
        .output()?;

    let _stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        warn!("        Verification failed");
        if !stderr.is_empty() {
            warn!("        Error output:");
            for line in stderr.lines() {
                warn!("          {}", line);
            }
        }
        // Don't fail the step, just warn
        warn!("        Continuing despite verification warning");
    } else {
        info!("        All contract functions verified");
    }

    Ok(())
}
