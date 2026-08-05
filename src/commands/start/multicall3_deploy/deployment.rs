//! Multicall3 deployment logic.
//!
//! This module contains the core deployment functionality for the Multicall3 contract.

use super::key_management;
use super::prerequisites::check_required_addresses;
use crate::commands::start::lotus_utils::get_lotus_rpc_url;
use crate::docker::command_logger::run_and_log_command_strings;
use crate::docker::push_bind_mount;
use crate::paths::foc_devnet_multicall3_repo;
use std::error::Error;
use tracing::{error, info};

/// Deploy Multicall3 using forge create
pub fn deploy_multicall3(
    private_key: &str,
    lotus_rpc_url: &str,
    run_id: &str,
    context: &super::super::step::SetupContext,
) -> Result<String, Box<dyn Error>> {
    info!("Deploying Multicall3 contract...");

    // Get the multicall3 repository path
    let multicall3_repo = foc_devnet_multicall3_repo();

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

    info!("Compiling and deploying contract...");

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

    let container_name = format!("foc-{}-multicall3-deploy", run_id);
    let mut args = vec![
        "run".to_string(),
        "-u".to_string(),
        "foc-user".to_string(),
        "--name".to_string(),
        container_name,
        "--network".to_string(),
        "host".to_string(), // Use host network to access Lotus RPC on dynamic port
    ];
    push_bind_mount(&mut args, &multicall3_repo, "/workspace")?;
    args.extend([
        crate::constants::BUILDER_DOCKER_IMAGE.to_string(),
        "bash".to_string(),
        "-c".to_string(),
        deploy_cmd,
    ]);

    let key = format!("multicall3_deploy_{}", run_id);
    let output = run_and_log_command_strings("docker", &args, context, &key)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print output for debugging
    if !stdout.is_empty() {
        info!("Deployment output:");
        for line in stdout.lines() {
            info!("{}", line);
        }
    }

    if !output.status.success() {
        error!(" ✗ Deployment failed");
        if !stderr.is_empty() {
            error!(" Error output:");
            for line in stderr.lines() {
                error!(" {}", line);
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

    info!("✓ Multicall3 deployed at: {}", contract_address);

    Ok(contract_address.to_string())
}

/// Perform the Multicall3 deployment process
pub fn perform_deployment(
    _volumes_dir: &std::path::PathBuf,
    context: &super::super::step::SetupContext,
) -> Result<(), Box<dyn Error>> {
    info!("Deploying Multicall3 contract...");

    // Get required addresses from context
    let (multicall3_deployer, multicall3_deployer_eth) = check_required_addresses(context)?;

    // Get deployer private key from addresses.json
    let private_key = key_management::get_deployer_private_key(&multicall3_deployer)?;

    info!("Deployer ETH address: {}", multicall3_deployer_eth);

    // Deploy Multicall3 contract
    let lotus_rpc_url = get_lotus_rpc_url(context)?;
    let run_id = context.run_id();
    let multicall3_address = deploy_multicall3(&private_key, &lotus_rpc_url, run_id, context)?;

    // Store in context
    context.set("multicall3_address", &multicall3_address);

    // Load existing contract addresses and add multicall3
    let mut addresses_struct =
        crate::commands::start::foc_deploy::contract_addresses::ContractAddresses::load(run_id)
            .unwrap_or_else(|_| {
                // If no existing addresses, create a minimal struct
                // This shouldn't happen as multicall3 runs after other deployments
                crate::commands::start::foc_deploy::contract_addresses::ContractAddresses::default()
            });

    // Add multicall3 to contracts
    addresses_struct
        .contracts
        .insert("multicall".to_string(), multicall3_address.clone());

    // Save updated addresses
    addresses_struct.save(run_id)?;

    // Verify the deployment
    super::verification::verify_multicall3(
        &private_key,
        &multicall3_address,
        &lotus_rpc_url,
        context,
    )?;

    info!(
        "✓ Multicall3 contract deployed successfully! Address: {}",
        multicall3_address
    );

    Ok(())
}
