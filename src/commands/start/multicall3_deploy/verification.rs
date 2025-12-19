//! Contract verification for Multicall3 deployment.
//!
//! This module handles the verification of deployed Multicall3 contracts.

use std::error::Error;
use std::process::Command;
use tracing::{info, warn};

/// Verify the deployed Multicall3 contract
pub fn verify_multicall3(
    _private_key: &str,
    contract_address: &str,
    lotus_rpc_url: &str,
) -> Result<(), Box<dyn Error>> {
    info!("Verifying Multicall3 contract functions...");

    // Wait a bit for transaction confirmation
    info!("Waiting for transaction confirmation...");
    std::thread::sleep(std::time::Duration::from_secs(6));

    // Verify that the contract exists at the address using cast
    let verify_cmd = format!("cast code {} --rpc-url {}", contract_address, lotus_rpc_url);

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            "foc-builder",
            "bash",
            "-c",
            &verify_cmd,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        warn!("⚠ Verification failed");
        if !stderr.is_empty() {
            info!("Error output:");
            for line in stderr.lines() {
                info!("{}", line);
            }
        }
        info!("→ Continuing despite verification warning");
        return Ok(());
    }

    if stdout.trim() == "0x" || stdout.trim().is_empty() {
        warn!(
            "        ⚠ No contract code found at address {}",
            contract_address
        );
        info!("→ Continuing despite verification warning");
        return Ok(());
    }

    info!(
        "        ✓ Multicall3 contract code verified at {}",
        contract_address
    );
    Ok(())
}
