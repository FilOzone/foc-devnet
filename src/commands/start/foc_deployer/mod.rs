//! FOC contract deployment logic.
//!
//! This module contains the core logic for deploying FOC contracts,
//! including private key extraction, deployment script execution,
//! and output parsing.

use super::foc_metadata::FOCMetadata;
use crate::constants::*;
use crate::docker::core::docker_command;
use crate::paths::{foc_devnet_bin, foc_devnet_docker_volumes_cache};
use std::error::Error;
use tracing::{info, warn};

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

/// Get the private key for an address in hex format (for use with cast/forge)
///
/// # Arguments
/// * `address` - Either a Filecoin address (t4...) or Ethereum address (0x...)
/// * `_lotus_container` - Unused, kept for API compatibility
///
/// # Returns
/// The private key as a hex string with 0x prefix
pub fn get_private_key(address: &str, _lotus_container: &str) -> Result<String, Box<dyn Error>> {
    // Load pre-generated keys
    let keys = crate::commands::init::keys::load_keys()?;

    // Determine if the address is Ethereum (0x...) or Filecoin (t4...)
    let key_info = if address.starts_with("0x") || address.starts_with("0X") {
        // Search by Ethereum address
        keys.iter()
            .find(|k| {
                k.eth_address
                    .as_ref()
                    .map(|eth| eth.eq_ignore_ascii_case(address))
                    .unwrap_or(false)
            })
            .ok_or(format!(
                "Private key not found for Ethereum address: {}",
                address
            ))?
    } else {
        // Search by Filecoin address (t4...)
        keys.iter()
            .find(|k| k.filecoin_address.as_ref() == Some(&address.to_string()))
            .ok_or(format!(
                "Private key not found for Filecoin address: {}",
                address
            ))?
    };

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
    run_id: &str,
) -> Result<DeploymentResult, Box<dyn Error>> {
    info!("Running deploy-all-warm-storage.sh...");

    // Log the RPC URL for debugging
    info!("Lotus RPC URL: {}", lotus_rpc_url);

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

    let bin_dir = foc_devnet_bin();
    let builder_volumes_dir =
        foc_devnet_docker_volumes_cache().join(crate::constants::BUILDER_CONTAINER);

    // Get the private key from lotus for the deployer address
    let private_key = get_private_key(foc_deployer, lotus_container)?;

    // Prepare environment variables for the deployment script
    let env_vars = [
        ("ETH_RPC_URL", lotus_rpc_url.to_string()),
        ("USDFC_TOKEN_ADDRESS", mock_usdfc_address.to_string()),
        ("SERVICE_NAME", "FOC DevNet Warm Storage".to_string()),
        (
            "SERVICE_DESCRIPTION",
            "Warm storage service for FOC local development network".to_string(),
        ),
        ("DRY_RUN", "false".to_string()),
        ("CHAIN", LOCAL_NETWORK_CHAIN_ID.to_string()),
        ("DEPLOYER_ADDRESS", deployer_eth_addr.to_string()),
        ("AUTO_VERIFY", "false".to_string()),
        ("ETH_PRIVATE_KEY", private_key.clone()),
        ("PASSWORD", "".to_string()),
        (
            "ETH_KEYSTORE",
            "/home/foc-user/.foundry/keystores/foc-deployer".to_string(),
        ),
    ];

    // Run the deployment script
    // Import wallet into keystore first (required by deploy-all-warm-storage.sh)
    // The script uses forge create --password which requires a keystore file
    let deploy_cmd = format!(
        r#"set -e
mkdir -p /home/foc-user/.foundry/keystores
cast wallet import foc-deployer --private-key {} --unsafe-password ''
cd /service_contracts
bash /service_contracts/tools/deploy-all-warm-storage.sh 2>&1 | tee /tmp/foc-deploy.log"#,
        private_key
    );

    info!("This may take several minutes...");

    let container_name = format!("foc-{}-foc-deploy", run_id);

    let mut docker_args = vec![
        "run".to_string(),
        "-u".to_string(),
        "foc-user".to_string(),
        "--name".to_string(),
        container_name.clone(),
        "--network".to_string(),
        "host".to_string(),
    ];

    // Add environment variables
    for (key, value) in env_vars {
        docker_args.push("-e".to_string());
        docker_args.push(format!("{}={}", key, value));
    }

    // Add volumes
    docker_args.push("-v".to_string());
    docker_args.push(format!("{}:/opt/bin", bin_dir.display()));
    docker_args.push("-v".to_string());
    docker_args.push(format!(
        "{}:/home/foc-user/.cargo",
        builder_volumes_dir.join("cargo").display()
    ));
    docker_args.push("-v".to_string());
    docker_args.push(format!("{}:/service_contracts", contracts_dir.display()));

    // Add image and command
    docker_args.push(BUILDER_DOCKER_IMAGE.to_string());
    docker_args.push("/bin/bash".to_string());
    docker_args.push("-c".to_string());
    docker_args.push(deploy_cmd);

    let args_ref: Vec<&str> = docker_args.iter().map(|s| s.as_str()).collect();

    info!("Executing deployment container: {}", container_name);
    info!(
        "Docker command: docker run [args with {} total args]",
        args_ref.len()
    );

    // Log the command line length for debugging
    let total_cmd_len: usize = args_ref.iter().map(|s| s.len()).sum();
    info!("Total command line length: {} bytes", total_cmd_len);

    let output = docker_command(&args_ref)?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        warn!(
            "Deployment container failed with exit status: {:?}",
            output.status.code()
        );

        // Print stderr
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        if !stderr_str.is_empty() {
            warn!("=== STDERR ===");
            for line in stderr_str.lines() {
                warn!("{}", line);
            }
        }

        // Print stdout as well
        if !output_str.is_empty() {
            warn!("=== STDOUT ===");
            for line in output_str.lines() {
                warn!("{}", line);
            }
        }

        // Try to get container logs if the container still exists
        if let Ok(logs) = crate::docker::core::get_container_logs(&container_name) {
            if !logs.is_empty() {
                warn!("=== CONTAINER LOGS ===");
                for line in logs.lines() {
                    warn!("{}", line);
                }
            }
        } else {
            info!(
                "Container {} does not exist or logs not accessible",
                container_name
            );
        }

        // Also try to inspect the container for more info
        let inspect_output = crate::docker::core::docker_command(&["inspect", &container_name]);
        if let Ok(output) = inspect_output {
            let inspect_str = String::from_utf8_lossy(&output.stdout);
            warn!("=== CONTAINER INSPECT ===");
            for line in inspect_str.lines() {
                warn!("{}", line);
            }
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
    let mut network_name = String::from("devnet");
    let mut challenge_finality = String::new();
    let mut max_proving_period = String::new();
    let mut challenge_window_size = String::new();
    let mut service_name = String::new();
    let mut service_description = String::new();

    info!("Parsing deployment output for contract addresses...");

    // Look for "DEPLOYMENT SUMMARY" section
    let mut in_summary = false;
    let mut in_network_config = false;

    for line in output_str.lines() {
        // Parse "SessionKeyRegistry deployed at 0x..." (appears before summary)
        if line.contains("SessionKeyRegistry deployed at") {
            if let Some(addr) = extract_address_from_deployed_line(line) {
                info!("Found SessionKeyRegistry: {}", addr);
                addresses.insert("session_key_registry".to_string(), addr);
                continue;
            }
        }

        if line.contains("DEPLOYMENT SUMMARY") {
            in_summary = true;
            info!("Found DEPLOYMENT SUMMARY section");
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
                    info!("Found contract: {} -> {}", snake_case_name, addr);
                    addresses.insert(snake_case_name, addr.to_string());
                } else {
                    info!("Skipping line with invalid address: {}", addr);
                }
            } else {
                info!(
                    "Skipping line that doesn't split into exactly 2 parts: {}",
                    line
                );
            }
        }

        // Check for Network Configuration section
        if line.contains("Network Configuration") {
            in_network_config = true;
            in_summary = false;
            info!("Found Network Configuration section");

            // Extract network name from "Network Configuration (devnet):"
            if let Some(start) = line.find('(') {
                if let Some(end) = line.find(')') {
                    network_name = line[start + 1..end].to_string();
                    info!("Network name: {}", network_name);
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
                        info!("Challenge finality: {}", challenge_finality);
                    }
                    "Max proving period" => {
                        max_proving_period = value.replace(" epochs", "").trim().to_string();
                        info!("Max proving period: {}", max_proving_period);
                    }
                    "Challenge window size" => {
                        challenge_window_size = value.replace(" epochs", "").trim().to_string();
                        info!("Challenge window size: {}", challenge_window_size);
                    }
                    "USDFC token address" => {
                        // Already captured in addresses
                        info!("USDFC token address: {}", value);
                    }
                    "FilBeam controller address" => {
                        filbeam_controller = Some(value.clone());
                        info!("FilBeam controller: {}", value);
                    }
                    "FilBeam beneficiary address" => {
                        filbeam_beneficiary = Some(value.clone());
                        info!("FilBeam beneficiary: {}", value);
                    }
                    "Service name" => {
                        service_name = value.clone();
                        info!("Service name: {}", value);
                    }
                    "Service description" => {
                        service_description = value.clone();
                        info!("Service description: {}", value);
                    }
                    _ => {}
                }
            }
        }
    }

    if addresses.is_empty() {
        warn!("No contract addresses found in output");
        info!("Full output:");
        for line in output_str.lines() {
            info!("{}", line);
        }
        info!("Deployment may have failed or output format changed");
    } else {
        info!(
            "Successfully parsed {} contracts from output",
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

/// Extract an address from a "<Name> deployed at 0x..." line.
///
/// # Example
/// ```text
/// SessionKeyRegistry deployed at 0xaF69542d01111EdfB7B63Aa974E6A2c9A31EA1E9
/// ```
fn extract_address_from_deployed_line(line: &str) -> Option<String> {
    let marker = "deployed at ";
    let idx = line.find(marker)?;
    let addr = line[idx + marker.len()..].trim();
    if addr.starts_with("0x") && addr.len() >= 42 {
        Some(addr.to_string())
    } else {
        None
    }
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
