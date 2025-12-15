//! MockUSDFC deployment logic.
//!
//! This module contains the core deployment functionality for the MockUSDFC token.

use super::foundry_setup::{get_mockusdfc_project_dir, setup_foundry_project};
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

// Network configuration
const LOTUS_RPC_PORT: u16 = 1234;

/// Deploy MockUSDFC using the Foundry project
pub fn deploy_mock_usdfc_foundry(
    private_key: &str,
    lotus_rpc_url: &str,
) -> Result<String, Box<dyn Error>> {
    println!("      Deploying MockUSDFC using Foundry project...");

    // Get the contract directory from embedded assets
    let contract_dir = get_mockusdfc_project_dir()?;

    // Setup the Foundry project (install deps, build)
    setup_foundry_project(&contract_dir)?;

    // Deploy using forge script with explicit gas limit for FEVM
    println!("      Executing deployment script...");

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
            "--network",
            "host", // Use host network to access localhost:1234
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
    if !stdout.is_empty() {
        println!("        Deployment output:");
        for line in stdout.lines() {
            println!("          {}", line);
        }
    }

    if !output.status.success() {
        println!("        {} Deployment failed", "✗".red());
        if !stderr.is_empty() {
            println!("        Error output:");
            for line in stderr.lines() {
                println!("          {}", line);
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

    println!(
        "        {} MockUSDFC deployed at: {}",
        "✓".green(),
        contract_address.cyan().bold()
    );

    Ok(contract_address.to_string())
}

/// Perform the MockUSDFC deployment process
pub fn perform_token_deployment(
    volumes_dir: &std::path::PathBuf,
    context: &mut super::super::step::StepContext,
) -> Result<(), Box<dyn Error>> {
    use super::key_management::get_deployer_private_key;
    use super::prerequisites::check_required_addresses;

    println!("    Deploying MockUSDFC token using Foundry project...");

    // Get required addresses from context
    let (mockusdfc_deployer, mockusdfc_deployer_eth) = check_required_addresses(context)?;

    // Get deployer private key from the exported key file
    let private_key = get_deployer_private_key(volumes_dir, &mockusdfc_deployer)?;

    println!(
        "      Deployer ETH address: {}",
        mockusdfc_deployer_eth.cyan()
    );

    // Deploy MockUSDFC token using Foundry
    let lotus_rpc_url = format!("http://localhost:{}/rpc/v1", LOTUS_RPC_PORT);
    let mock_usdfc_address = deploy_mock_usdfc_foundry(&private_key, &lotus_rpc_url)?;

    // Store in context
    context.set("mock_usdfc_address", &mock_usdfc_address);

    // Save to contract addresses file
    super::contract_storage::save_contract_address("usdfc", &mock_usdfc_address)?;

    // Verify the deployment
    super::verification::verify_mock_usdfc(&private_key, &mock_usdfc_address, &lotus_rpc_url)?;

    println!(
        "\n    {} MockUSDFC token deployed successfully!",
        "✓".green().bold()
    );
    println!("      Token Address: {}", mock_usdfc_address.cyan().bold());
    println!(
        "      Initial Supply: {} tokens",
        super::usdfc_deploy_step::MOCK_USDFC_INITIAL_SUPPLY
    );
    println!("      Decimals: 18");

    Ok(())
}
