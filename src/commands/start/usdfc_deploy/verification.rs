//! Contract verification for MockUSDFC deployment.
//!
//! This module handles the verification of deployed MockUSDFC contracts.

use super::foundry_setup::get_mockusdfc_project_dir;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

/// Verify the deployed MockUSDFC contract
pub fn verify_mock_usdfc(
    private_key: &str,
    contract_address: &str,
    lotus_rpc_url: &str,
) -> Result<(), Box<dyn Error>> {
    println!("      Verifying MockUSDFC contract functions...");

    // Get the contract directory from embedded assets
    let contract_dir = get_mockusdfc_project_dir()?;

    // Wait a bit for transaction confirmation
    println!("        Waiting for transaction confirmation...");
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print verification output
    if !stdout.is_empty() {
        println!("        Verification output:");
        for line in stdout.lines() {
            println!("          {}", line);
        }
    }

    if !output.status.success() {
        println!("        {} Verification failed", "⚠".yellow());
        if !stderr.is_empty() {
            println!("        Error output:");
            for line in stderr.lines() {
                println!("          {}", line);
            }
        }
        // Don't fail the step, just warn
        println!(
            "        {} Continuing despite verification warning",
            "→".cyan()
        );
    } else {
        println!("        {} All contract functions verified", "✓".green());
    }

    Ok(())
}
