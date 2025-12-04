//! Multicall3 deployment logic.
//!
//! This module contains the core deployment functionality for the Multicall3 contract.

use crate::paths::foc_localnet_multicall3_repo;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

// Network configuration
const LOTUS_RPC_PORT: u16 = 1234;

/// Deploy Multicall3 using forge create
pub fn deploy_multicall3(private_key: &str, lotus_rpc_url: &str) -> Result<String, Box<dyn Error>> {
    println!("      Deploying Multicall3 contract...");

    // Get the multicall3 repository path
    let multicall3_repo = foc_localnet_multicall3_repo();

    if !multicall3_repo.exists() {
        return Err(format!(
            "Multicall3 repository not found at: {}",
            multicall3_repo.display()
        )
        .into());
    }

    // Check if Multicall3.sol exists in the repo
    let contract_file = multicall3_repo.join("src/Multicall3.sol");
    if !contract_file.exists() {
        return Err(format!("Multicall3.sol not found at: {}", contract_file.display()).into());
    }

    println!("      Compiling and deploying contract...");

    // Deploy using forge create with explicit gas limit for FEVM
    let deploy_cmd = format!(
        "cd /workspace && \
         forge create src/Multicall3.sol:Multicall3 \
         --rpc-url {} \
         --private-key {} \
         --legacy \
         --gas-limit 1000000000 \
         --broadcast \
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
            &format!("{}:/workspace", multicall3_repo.display()),
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
        return Err("Multicall3 deployment failed".into());
    }

    // Extract contract address from output
    // Look for "Deployed to:" in the output
    let contract_address = stdout
        .lines()
        .find(|line| line.contains("Deployed to:"))
        .and_then(|line| line.split_whitespace().last())
        .ok_or("Failed to extract contract address from deployment output")?;

    println!(
        "        {} Multicall3 deployed at: {}",
        "✓".green(),
        contract_address.cyan().bold()
    );

    Ok(contract_address.to_string())
}

/// Perform the Multicall3 deployment process
pub fn perform_deployment(
    volumes_dir: &std::path::PathBuf,
    context: &mut super::super::step::StepContext,
) -> Result<(), Box<dyn Error>> {
    use super::key_management::get_deployer_private_key;
    use super::prerequisites::check_required_addresses;

    println!("    Deploying Multicall3 contract...");

    // Get required addresses from context
    let (multicall3_deployer, multicall3_deployer_eth) = check_required_addresses(context)?;

    // Get deployer private key from the exported key file
    let private_key = get_deployer_private_key(volumes_dir, &multicall3_deployer)?;

    println!(
        "      Deployer ETH address: {}",
        multicall3_deployer_eth.cyan()
    );

    // Deploy Multicall3 contract
    let lotus_rpc_url = format!("http://localhost:{}/rpc/v1", LOTUS_RPC_PORT);
    let multicall3_address = deploy_multicall3(&private_key, &lotus_rpc_url)?;

    // Store in context
    context.set("multicall3_address", &multicall3_address);

    // Save to contract addresses file
    super::contract_storage::save_contract_address("Multicall3", &multicall3_address)?;

    // Verify the deployment
    super::verification::verify_multicall3(&private_key, &multicall3_address, &lotus_rpc_url)?;

    println!(
        "\n    {} Multicall3 contract deployed successfully!",
        "✓".green().bold()
    );
    println!(
        "      Contract Address: {}",
        multicall3_address.cyan().bold()
    );

    Ok(())
}
