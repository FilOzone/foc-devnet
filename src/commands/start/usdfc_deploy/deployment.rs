//! MockUSDFC deployment logic.
//!
//! This module contains the core deployment functionality for the MockUSDFC token.

use super::foundry_setup::{get_mockusdfc_project_dir, setup_foundry_project};
use super::key_management::get_deployer_private_key;
use super::prerequisites::check_required_addresses;
use crate::commands::start::lotus_utils::get_lotus_rpc_url;
use crate::commands::start::step::SetupContext;
use crate::docker::bind_mount;
use crate::docker::command_logger::run_and_log_command;
use std::error::Error;
use std::path::PathBuf;
use tracing::{error, info};

/// Build the forge deploy command.
///
/// `INITIAL_SUPPLY` is passed explicitly from the Rust constant so the deployed
/// token always matches the supply that funding allocates against. Relying on
/// the Solidity script's fallback default would let the two values drift apart.
fn mock_usdfc_deploy_command(lotus_rpc_url: &str, private_key: &str) -> String {
    format!(
        "cd /workspace && \
         INITIAL_SUPPLY={} \
         forge script script/Deploy.s.sol:DeployMockUSDFC \
         --rpc-url {} \
         --private-key {} \
         --broadcast \
         --slow \
         --gas-estimate-multiplier 10000 \
         -vv",
        crate::constants::MOCK_USDFC_INITIAL_SUPPLY,
        lotus_rpc_url,
        private_key
    )
}

/// Deploy MockUSDFC using the Foundry project
pub fn deploy_mock_usdfc_foundry(
    context: &SetupContext,
    private_key: &str,
    lotus_rpc_url: &str,
    run_id: &str,
) -> Result<(String, PathBuf), Box<dyn Error>> {
    info!("Deploying MockUSDFC using Foundry project...");

    // Get the contract directory from embedded assets
    let contract_dir = get_mockusdfc_project_dir(run_id)?;

    // Setup the Foundry project (install deps, build)
    setup_foundry_project(context, &contract_dir, run_id)?;

    // Deploy using forge script with explicit gas limit for FEVM
    info!("Executing deployment script...");

    let deploy_cmd = mock_usdfc_deploy_command(lotus_rpc_url, private_key);

    let key = format!("usdfc_deploy_{}", run_id);
    let contract_mount = bind_mount(&contract_dir, "/workspace")?;
    let output = run_and_log_command(
        "docker",
        &[
            "run",
            "-u",
            "foc-user",
            "--name",
            &format!("foc-{}-usdfc-deploy", run_id),
            "--network",
            "host", // Use host network to access Lotus RPC on dynamic port
            "--mount",
            &contract_mount,
            crate::constants::BUILDER_DOCKER_IMAGE,
            "bash",
            "-c",
            &deploy_cmd,
        ],
        context,
        &key,
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print output for debugging
    // if !stdout.is_empty() {
    //     println!("Deployment output:");
    //     for line in stdout.lines() {
    //         println!("  {}", line);
    //     }
    // }

    if !output.status.success() {
        error!(" ✗ Deployment failed");
        if !stderr.is_empty() {
            error!(" Error output:");
            for line in stderr.lines() {
                error!(" {}", line);
            }
        }
        return Err("MockUSDFC deployment failed".into());
    }

    // Extract contract address from output
    // Look for "MockUSDFC deployed at:" in the output
    let contract_address = stdout
        .lines()
        .find(|line| line.contains("MockUSDFC deployed at:"))
        .and_then(|line| line.split_whitespace().last())
        .ok_or("Failed to extract contract address from deployment output")?;

    info!("✓ MockUSDFC deployed at: {}", contract_address);

    Ok((contract_address.to_string(), contract_dir))
}

/// Perform the MockUSDFC deployment process
pub fn perform_token_deployment(
    _volumes_dir: &std::path::PathBuf,
    context: &super::super::step::SetupContext,
) -> Result<(), Box<dyn Error>> {
    info!("Deploying MockUSDFC token using Foundry project...");

    // Get required addresses from context
    let (mockusdfc_deployer, mockusdfc_deployer_eth) = check_required_addresses(context)?;

    // Get deployer private key from addresses.json
    let private_key = get_deployer_private_key(&mockusdfc_deployer)?;

    info!("Deployer ETH address: {}", mockusdfc_deployer_eth);

    // Get Lotus RPC URL
    let lotus_rpc_url = get_lotus_rpc_url(context)?;
    let run_id = context.run_id();

    // Deploy MockUSDFC
    let (mock_usdfc_address, contract_dir) =
        deploy_mock_usdfc_foundry(context, &private_key, &lotus_rpc_url, run_id)?;

    // Store in context
    context.set("mockusdfc_contract_address", &mock_usdfc_address);

    // Save to contract addresses file
    super::contract_storage::save_contract_address(run_id, "usdfc", &mock_usdfc_address)?;

    // Verify the deployment
    super::verification::verify_mock_usdfc(
        context,
        &private_key,
        &mock_usdfc_address,
        &lotus_rpc_url,
        run_id,
        &contract_dir,
    )?;

    info!("✓ MockUSDFC token deployed successfully!");
    info!("Token Address: {}", mock_usdfc_address);
    info!(
        "Initial Supply: {} tokens (base units: {})",
        crate::constants::MOCK_USDFC_INITIAL_SUPPLY_TOKENS,
        crate::constants::MOCK_USDFC_INITIAL_SUPPLY
    );
    info!("Decimals: 18");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deploy_command_sets_initial_supply_from_shared_constant() {
        let cmd = mock_usdfc_deploy_command("http://localhost:1234/rpc/v1", "0xabc");

        assert!(
            cmd.contains(&format!(
                "INITIAL_SUPPLY={}",
                crate::constants::MOCK_USDFC_INITIAL_SUPPLY
            )),
            "command must pass the shared supply constant: {}",
            cmd
        );
        assert!(cmd.contains("script/Deploy.s.sol:DeployMockUSDFC"));
        assert!(cmd.contains("--rpc-url http://localhost:1234/rpc/v1"));
        assert!(cmd.contains("--private-key 0xabc"));
    }
}
