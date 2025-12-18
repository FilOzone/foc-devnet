//! FOC contract deployment logic.
//!
//! This module contains the core logic for deploying FOC contracts,
//! including private key extraction, deployment script execution,
//! and output parsing.

use super::foc_metadata::FOCMetadata;
use crate::constants::*;
use crate::docker::core::docker_command;
use crate::paths::{foc_localnet_bin, foc_localnet_docker_volumes};
use crossterm::style::Stylize;
use std::error::Error;

/// Deployment result containing contract addresses and network metadata
pub struct DeploymentResult {
    /// Map of contract names to addresses
    pub addresses: std::collections::HashMap<String, String>,
    /// FilBeam controller address
    pub filbeam_controller: Option<String>,
    /// FilBeam beneficiary address
    pub filbeam_beneficiary: Option<String>,
    /// Network configuration metadata
    pub metadata: FOCMetadata,
}

/// Get the private key for an f4 address in hex format (for use with cast/forge)
pub fn get_private_key(f4_address: &str, _lotus_container: &str) -> Result<String, Box<dyn Error>> {
    // Load pre-generated keys
    let keys = crate::commands::init::keys::load_keys()?;

    // Find the key with matching Filecoin address
    let key_info = keys
        .iter()
        .find(|k| k.filecoin_address.as_ref() == Some(&f4_address.to_string()))
        .ok_or(format!("Private key not found for address: {}", f4_address))?;

    // Return the private key with 0x prefix
    Ok(format!("0x{}", key_info.private_key))
}

/// Deploy FOC contracts using the deployment script
///
/// Returns deployment result with contract addresses and network metadata
pub fn deploy_foc_contracts(
    foc_deployer: &str,
    deployer_eth_addr: &str,
    mock_usdfc_address: &str,
    services_repo_path: &std::path::Path,
    lotus_container: &str,
    lotus_rpc_url: &str,
) -> Result<DeploymentResult, Box<dyn Error>> {
    println!("      Running deploy-all-warm-storage.sh...");

    // Resolve symlinks to get the real path for Docker mounting
    let services_repo = services_repo_path
        .canonicalize()
        .unwrap_or_else(|_| services_repo_path.to_path_buf());
    let contracts_dir = services_repo.join("service_contracts");
    let deploy_script = contracts_dir
        .join("tools")
        .join("deploy-all-warm-storage.sh");

    if !deploy_script.exists() {
        return Err(format!("Deployment script not found at {}", deploy_script.display()).into());
    }

    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    // Get the private key from lotus for the deployer address
    let private_key = get_private_key(foc_deployer, lotus_container)?;

    // Prepare environment variables for the deployment script
    let env_vars = format!(
        r#"export ETH_RPC_URL='{}'
export USDFC_TOKEN_ADDRESS='{}'
export SERVICE_NAME='FOC LocalNet Warm Storage'
export SERVICE_DESCRIPTION='Warm storage service for FOC local development network'
export DRY_RUN=false
export CHAIN={}
export DEPLOYER_ADDRESS='{}'
export AUTO_VERIFY=false
export ETH_PRIVATE_KEY='{}'
export PASSWORD=''"#,
        lotus_rpc_url, mock_usdfc_address, LOCAL_NETWORK_CHAIN_ID, deployer_eth_addr, private_key
    );

    // Run the deployment script
    // First, create a keystore from the private key with empty password
    let deploy_cmd = format!(
        r#"set -e
cast wallet import foc-deployer --private-key {} --unsafe-password ''
export ETH_KEYSTORE="$HOME/.foundry/keystores/foc-deployer"
{}
cd /service_contracts
bash /service_contracts/tools/deploy-all-warm-storage.sh 2>&1 | tee /tmp/foc-deploy.log"#,
        private_key, env_vars
    );

    println!("        This may take several minutes...");

    let output = docker_command(&[
        "run",
        "--rm",
        "--network",
        "host",
        "-v",
        &format!("{}:/opt/bin", bin_dir.display()),
        "-v",
        &format!(
            "{}:/home/foc-user/.cargo",
            builder_volumes_dir.join("cargo").display()
        ),
        "-v",
        &format!("{}:/service_contracts", contracts_dir.display()),
        BUILDER_CONTAINER,
        "/bin/bash",
        "-c",
        &deploy_cmd,
    ])?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Print output for debugging
    // println!("\n        Deployment output:");
    // for line in output_str.lines() {
    //     println!("          {}", line);
    // }

    if !output.status.success() {
        println!("        {} Deployment script failed", "✗".red());
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        println!("        Error output:");
        for line in stderr_str.lines() {
            println!("          {}", line);
        }
        return Err("FOC contract deployment failed".into());
    }

    // Parse the deployment output to extract contract addresses and metadata
    let deployment_result = parse_deployment_output(&output_str)?;

    Ok(deployment_result)
}

/// Parse deployment output to extract contract addresses and network metadata
pub fn parse_deployment_output(output_str: &str) -> Result<DeploymentResult, Box<dyn Error>> {
    // Look for "DEPLOYMENT SUMMARY" section
    let mut addresses = std::collections::HashMap::new();
    let mut filbeam_controller = None;
    let mut filbeam_beneficiary = None;
    let mut network_name = String::from("localnet");
    let mut challenge_finality = String::new();
    let mut max_proving_period = String::new();
    let mut challenge_window_size = String::new();
    let mut service_name = String::new();
    let mut service_description = String::new();

    println!("        Parsing deployment output for contract addresses...");

    // Look for "DEPLOYMENT SUMMARY" section
    let mut in_summary = false;
    let mut in_network_config = false;

    for line in output_str.lines() {
        // println!("        Line: {}", line); // Debug: print each line

        if line.contains("DEPLOYMENT SUMMARY") {
            in_summary = true;
            println!("        Found DEPLOYMENT SUMMARY section");
            continue;
        }

        if in_summary && line.contains(":") && line.contains("0x") {
            // Parse lines like "PDPVerifier Implementation: 0x1234..."
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 2 {
                let name = parts[0].trim();
                let addr = parts[1].trim();
                if addr.starts_with("0x") && !addr.is_empty() {
                    // Convert name to snake_case for consistency
                    let snake_case_name = to_snake_case(name);
                    println!("        Found contract: {} -> {}", snake_case_name, addr);
                    addresses.insert(snake_case_name, addr.to_string());
                } else {
                    println!("        Skipping line with invalid address: {}", addr);
                }
            } else {
                println!(
                    "        Skipping line that doesn't split into exactly 2 parts: {}",
                    line
                );
            }
        }

        // Check for Network Configuration section
        if line.contains("Network Configuration") {
            in_network_config = true;
            in_summary = false;
            println!("        Found Network Configuration section");

            // Extract network name from "Network Configuration (localnet):"
            if let Some(start) = line.find('(') {
                if let Some(end) = line.find(')') {
                    network_name = line[start + 1..end].to_string();
                    println!("        Network name: {}", network_name);
                }
            }
            continue;
        }

        if in_network_config && line.contains(":") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let key = parts[0].trim();
                let value = parts[1..].join(":").trim().to_string();

                match key {
                    "Challenge finality" => {
                        challenge_finality = value.replace(" epochs", "").trim().to_string();
                        println!("        Challenge finality: {}", challenge_finality);
                    }
                    "Max proving period" => {
                        max_proving_period = value.replace(" epochs", "").trim().to_string();
                        println!("        Max proving period: {}", max_proving_period);
                    }
                    "Challenge window size" => {
                        challenge_window_size = value.replace(" epochs", "").trim().to_string();
                        println!("        Challenge window size: {}", challenge_window_size);
                    }
                    "USDFC token address" => {
                        // Already captured in addresses
                        println!("        USDFC token address: {}", value);
                    }
                    "FilBeam controller address" => {
                        filbeam_controller = Some(value.clone());
                        println!("        FilBeam controller: {}", value);
                    }
                    "FilBeam beneficiary address" => {
                        filbeam_beneficiary = Some(value.clone());
                        println!("        FilBeam beneficiary: {}", value);
                    }
                    "Service name" => {
                        service_name = value.clone();
                        println!("        Service name: {}", value);
                    }
                    "Service description" => {
                        service_description = value.clone();
                        println!("        Service description: {}", value);
                    }
                    _ => {}
                }
            }
        }
    }

    if addresses.is_empty() {
        println!(
            "        {} No contract addresses found in output",
            "⚠".yellow()
        );
        println!("        Full output:");
        for line in output_str.lines() {
            println!("          {}", line);
        }
        println!("        Deployment may have failed or output format changed");
    } else {
        println!(
            "        {} Successfully parsed {} contracts from output",
            "✓".green(),
            addresses.len()
        );
    }

    let metadata = FOCMetadata {
        network_name,
        challenge_finality,
        max_proving_period,
        challenge_window_size,
        service_name,
        service_description,
    };

    Ok(DeploymentResult {
        addresses,
        filbeam_controller,
        filbeam_beneficiary,
        metadata,
    })
}

/// Convert a contract name to snake_case
/// Examples:
/// - "FilecoinWarmStorageService Implementation" -> "filecoin_warm_storage_service_implementation"
/// - "ServiceProviderRegistry Proxy" -> "service_provider_registry_proxy"
/// - "FilecoinPayV1 Contract" -> "filecoin_pay_v1_contract"
fn to_snake_case(name: &str) -> String {
    let mut result = name
        .chars()
        .enumerate()
        .fold(String::new(), |mut acc, (i, c)| {
            if c.is_uppercase() && i > 0 {
                acc.push('_');
            }
            acc.push(c.to_lowercase().next().unwrap());
            acc
        })
        .replace(" ", "_");

    // Replace multiple consecutive underscores with single underscore
    while result.contains("__") {
        result = result.replace("__", "_");
    }

    result
}
