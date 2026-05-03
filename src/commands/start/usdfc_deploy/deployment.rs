//! MockUSDFC deployment logic.
//!
//! This module contains the core deployment functionality for the MockUSDFC token.

use super::foundry_setup::get_mockusdfc_project_dir;
use super::key_management::get_deployer_private_key;
use super::prerequisites::check_required_addresses;
use crate::commands::start::lotus_utils::get_lotus_rpc_url;
use crate::commands::start::step::SetupContext;
use crate::docker::command_logger::run_and_log_command;
use std::error::Error;
use std::path::PathBuf;
use tracing::{error, info};

const MOCK_USDFC_BYTECODE: &str =
    include_str!("../../../../contracts/MockUSDFC/bytecode/MockUSDFC.bin");

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

    // Deploy precompiled bytecode with explicit gas limit for FEVM. Keeping this
    // path compiler-free avoids startup-time downloads in ephemeral containers.
    info!("Creating MockUSDFC contract...");

    let deploy_cmd = format!(
        "cd /workspace && \
         CONSTRUCTOR_ARGS=$(cast abi-encode 'constructor(uint256)' {}) && \
         DEPLOY_BYTECODE={}$(echo $CONSTRUCTOR_ARGS | sed 's/^0x//') && \
         cast send --json \
         --rpc-url {} \
         --private-key {} \
         --gas-limit 1000000000 \
         --create \"$DEPLOY_BYTECODE\"",
        super::usdfc_deploy_step::MOCK_USDFC_INITIAL_SUPPLY,
        MOCK_USDFC_BYTECODE.trim(),
        lotus_rpc_url,
        private_key
    );

    let key = format!("usdfc_deploy_{}", run_id);
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
            "-v",
            &format!("{}:/workspace", contract_dir.display()),
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

    // Extract contract address from cast send JSON output.
    let receipt: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Failed to parse deployment JSON output: {}", e))?;
    let contract_address = receipt
        .get("contractAddress")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
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
        "Initial Supply: {} tokens",
        super::usdfc_deploy_step::MOCK_USDFC_INITIAL_SUPPLY
    );
    info!("Decimals: 18");

    Ok(())
}
