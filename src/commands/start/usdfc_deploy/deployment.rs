//! MockUSDFC deployment logic.
//!
//! This module contains the core deployment functionality for the MockUSDFC token.

use super::foundry_setup::{get_mockusdfc_project_dir, setup_foundry_project};
use super::key_management::get_deployer_private_key;
use super::prerequisites::check_required_addresses;
use crate::commands::start::lotus_utils::get_lotus_rpc_url;
use std::error::Error;
use std::process::Command;
use tracing::{error, info};

/// Deploy MockUSDFC using the Foundry project
pub fn deploy_mock_usdfc_foundry(
    private_key: &str,
    lotus_rpc_url: &str,
    run_id: &str,
) -> Result<String, Box<dyn Error>> {
    info!("      Deploying MockUSDFC using Foundry project...");

    // Get the contract directory from embedded assets
    let contract_dir = get_mockusdfc_project_dir(run_id)?;

    // Setup the Foundry project (install deps, build)
    setup_foundry_project(&contract_dir)?;

    // Deploy using forge script with explicit gas limit for FEVM
    info!("      Executing deployment script...");

    let deploy_cmd = format!(
        "cd /workspace && \
         forge script script/Deploy.s.sol:DeployMockUSDFC \
         --rpc-url {} \
         --private-key {} \
         --broadcast \
         --slow \
         --gas-estimate-multiplier 10000 \
         -vv",
        lotus_rpc_url, private_key
    );

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--name",
            &format!("foc-{}-usdfc-deploy", run_id),
            "--network",
            "host", // Use host network to access Lotus RPC on dynamic port
            "-v",
            &format!("{}:/workspace", contract_dir.display()),
            "foc-builder",
            "bash",
            "-c",
            &deploy_cmd,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print output for debugging
    // if !stdout.is_empty() {
    //     println!("        Deployment output:");
    //     for line in stdout.lines() {
    //         println!("          {}", line);
    //     }
    // }

    if !output.status.success() {
        error!("        ✗ Deployment failed");
        if !stderr.is_empty() {
            error!("        Error output:");
            for line in stderr.lines() {
                error!("          {}", line);
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

    info!("        ✓ MockUSDFC deployed at: {}", contract_address);

    Ok(contract_address.to_string())
}

/// Perform the MockUSDFC deployment process
pub fn perform_token_deployment(
    _volumes_dir: &std::path::PathBuf,
    context: &super::super::step::SetupContext,
) -> Result<(), Box<dyn Error>> {
    info!("    Deploying MockUSDFC token using Foundry project...");

    // Get required addresses from context
    let (mockusdfc_deployer, mockusdfc_deployer_eth) = check_required_addresses(context)?;

    // Get deployer private key from addresses.json
    let private_key = get_deployer_private_key(&mockusdfc_deployer)?;

    info!("      Deployer ETH address: {}", mockusdfc_deployer_eth);

    // Get Lotus RPC URL
    let lotus_rpc_url = get_lotus_rpc_url(context)?;
    let run_id = context.run_id().ok_or("Run ID not found in context")?;

    // Deploy MockUSDFC
    let mock_usdfc_address = deploy_mock_usdfc_foundry(&private_key, &lotus_rpc_url, run_id)?;

    // Store in context
    context.set("mockusdfc_contract_address", &mock_usdfc_address);

    // Save to contract addresses file
    super::contract_storage::save_contract_address(run_id, "usdfc", &mock_usdfc_address)?;

    // Verify the deployment
    super::verification::verify_mock_usdfc(
        &private_key,
        &mock_usdfc_address,
        &lotus_rpc_url,
        run_id,
    )?;

    info!("    ✓ MockUSDFC token deployed successfully!");
    info!("      Token Address: {}", mock_usdfc_address);
    info!(
        "      Initial Supply: {} tokens",
        super::usdfc_deploy_step::MOCK_USDFC_INITIAL_SUPPLY
    );
    info!("      Decimals: 18");

    Ok(())
}
