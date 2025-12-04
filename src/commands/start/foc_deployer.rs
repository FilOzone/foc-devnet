//! FOC contract deployment logic.
//!
//! This module contains the core logic for deploying FOC contracts,
//! including private key extraction, deployment script execution,
//! and output parsing.

use crate::constants::*;
use crate::docker::core::docker_command;
use crate::paths::{foc_localnet_bin, foc_localnet_docker_volumes};
use crossterm::style::Stylize;
use std::error::Error;

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
/// Returns a map of contract names to addresses
pub fn deploy_foc_contracts(
    foc_deployer: &str,
    deployer_eth_addr: &str,
    mock_usdfc_address: &str,
    services_repo_path: &std::path::Path,
    lotus_container: &str,
) -> Result<std::collections::HashMap<String, String>, Box<dyn Error>> {
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

    let lotus_rpc_url = format!("http://localhost:{}/rpc/v1", LOTUS_RPC_PORT);

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
    println!("\n        Deployment output:");
    for line in output_str.lines() {
        println!("          {}", line);
    }

    if !output.status.success() {
        println!("        {} Deployment script failed", "✗".red());
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        println!("        Error output:");
        for line in stderr_str.lines() {
            println!("          {}", line);
        }
        return Err("FOC contract deployment failed".into());
    }

    // Parse the deployment output to extract contract addresses
    let addresses = parse_deployment_output(&output_str)?;

    Ok(addresses)
}

/// Parse deployment output to extract contract addresses
pub fn parse_deployment_output(
    output_str: &str,
) -> Result<std::collections::HashMap<String, String>, Box<dyn Error>> {
    // Look for "DEPLOYMENT SUMMARY" section
    let mut addresses = std::collections::HashMap::new();

    // Look for "DEPLOYMENT SUMMARY" section
    let mut in_summary = false;
    for line in output_str.lines() {
        if line.contains("DEPLOYMENT SUMMARY") {
            in_summary = true;
            continue;
        }

        if in_summary && line.contains(":") && line.contains("0x") {
            // Parse lines like "PDPVerifier Implementation: 0x1234..."
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 2 {
                let name = parts[0].trim();
                let addr = parts[1].trim();
                if addr.starts_with("0x") {
                    addresses.insert(name.to_string(), addr.to_string());
                }
            }
        }

        // Stop parsing after configuration section
        if in_summary && line.contains("Network Configuration") {
            break;
        }
    }

    if addresses.is_empty() {
        println!(
            "        {} No contract addresses found in output",
            "⚠".yellow()
        );
        println!("        Deployment may have failed or output format changed");
    } else {
        println!(
            "        {} Successfully deployed {} contracts",
            "✓".green(),
            addresses.len()
        );
    }

    Ok(addresses)
}
