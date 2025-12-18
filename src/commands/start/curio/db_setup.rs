//! Database setup for Curio PDP Service Providers.
//!
//! Handles:
//! - Base layer migration (curio config new-cluster)
//! - PDP layer configuration (curio config create)

use super::super::contract_addresses::ContractAddresses;
use super::super::step::StepContext;
use super::constants::{DB_SETUP_WAIT_SECS, PDP_LAYER_CONFIG_TEMPLATE};
use crate::commands::start::genesis::constants::PDP_SP_MINER_ID_START;
use crate::commands::start::lotus_utils::{build_fullnode_api_info, read_lotus_token};
use crate::docker::containers::lotus_container_name;
use crate::docker::core::docker_command;
use crate::docker::network::{lotus_network_name, pdp_miner_network_name};
use crate::paths::foc_localnet_bin;
use crate::paths::foc_localnet_docker_volumes;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Build FOC contract environment variables from contract addresses file
///
/// Reads deployed contract addresses from ~/.foc-localnet/state/contract_addresses.json
/// and builds environment variables for Curio to use.
pub fn build_foc_contract_env_vars(_context: &StepContext) -> Result<Vec<String>, Box<dyn Error>> {
    let mut env_vars = Vec::new();

    // Load contract addresses from file
    let addresses = ContractAddresses::load()?;

    // Get standard contracts
    if let Some(usdfc) = addresses.contracts.get("usdfc") {
        env_vars.push(format!("FOC_CONTRACT_USDFC={}", usdfc));
    }

    // Get FOC service contracts
    if let Some(fwss) = addresses
        .foc_contracts
        .get("filecoin_warm_storage_service_proxy")
    {
        env_vars.push(format!("FOC_CONTRACT_FWSS={}", fwss));
    }
    if let Some(multicall) = addresses.contracts.get("multicall") {
        env_vars.push(format!("FOC_CONTRACT_MULTICALL={}", multicall));
    }
    if let Some(pay) = addresses.foc_contracts.get("filecoin_pay_v1_contract") {
        env_vars.push(format!("FOC_CONTRACT_PAY={}", pay));
    }
    if let Some(pdp) = addresses.foc_contracts.get("p_d_p_verifier_proxy") {
        env_vars.push(format!("FOC_PDP_VERIFIER_PROXY={}", pdp));
    }

    // SIMPLE contract is always zero address
    env_vars.push("FOC_CONTRACT_SIMPLE=0x0000000000000000000000000000000000000000".to_string());

    Ok(env_vars)
}

/// Build Curio database environment variables for a specific PDP SP.
pub fn build_db_env_vars(
    context: &StepContext,
    sp_index: usize,
) -> Result<Vec<String>, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let yugabyte_name = format!("foc-{}-yugabyte-{}", run_id, sp_index);

    Ok(vec![
        "CURIO_DB_SSLMODE=disable".to_string(),
        format!("CURIO_DB_HOST={}", yugabyte_name),
        "CURIO_DB_PORT=5433".to_string(),
        "CURIO_DB_USER=yugabyte".to_string(),
        "CURIO_DB_PASSWORD=yugabyte".to_string(),
        "CURIO_DB_NAME=yugabyte".to_string(),
        "CURIO_DB_LOAD_BALANCE=false".to_string(),
    ])
}

/// Build Lotus environment variables for Curio to connect to Lotus daemon.
///
/// Sets:
/// - FULLNODE_API_INFO: JWT token and multiaddr to Lotus API (dns4 for Docker networking)
/// - LOTUS_PATH: Path to lotus-data directory (host-side shared volume)
pub fn build_lotus_env_vars(context: &StepContext) -> Result<Vec<String>, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let lotus_name = lotus_container_name(run_id);

    // Read Lotus API token from host
    let lotus_token = read_lotus_token()?;

    // Build FULLNODE_API_INFO with dns4 addressing for Docker network
    let fullnode_api_info = build_fullnode_api_info(&lotus_token, &lotus_name);

    // Get host-side lotus-data path for LOTUS_PATH
    let lotus_data_dir = "/lotus-data".to_string();

    Ok(vec![
        format!("FULLNODE_API_INFO={}", fullnode_api_info),
        format!("LOTUS_PATH={}", lotus_data_dir),
    ])
}

/// Setup Curio database for a specific PDP SP.
///
/// Steps:
/// 1. Run `curio config new-cluster t0XXXX` for base layer migration
/// 2. Run `curio config create --title pdp-only` with PDP layer config
pub fn setup_curio_database(context: &StepContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!(
        "    {} Setting up database for PDP SP {}...",
        "💾".cyan(),
        sp_index
    );

    // Calculate miner ID for this PDP SP
    let miner_id = format!("t0{}", PDP_SP_MINER_ID_START + (sp_index as u32) - 1);

    // Step 1: Base layer migration
    create_base_cluster(context, sp_index, &miner_id)?;

    // Step 2: PDP layer configuration
    create_pdp_layer(context, sp_index)?;

    println!(
        "    {} Database setup complete for PDP SP {}",
        "✓".green(),
        sp_index
    );

    Ok(())
}

/// Create base cluster configuration for a miner.
///
/// Runs: `curio config new-cluster <miner_id>` in a temporary container
fn create_base_cluster(
    context: &StepContext,
    sp_index: usize,
    miner_id: &str,
) -> Result<(), Box<dyn Error>> {
    println!(
        "      {} Creating base cluster for miner {}...",
        "⚙".cyan(),
        miner_id
    );

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let pdp_network = pdp_miner_network_name(run_id, sp_index);
    let lotus_network = lotus_network_name(run_id);

    // Get binary directory for volume mount
    let bin_dir = foc_localnet_bin();
    let bin_mount = format!("{}:/usr/local/bin/lotus-bins", bin_dir.display());

    // Get lotus-data directory for volume mount (needed for token and LOTUS_PATH)
    let lotus_data_dir = foc_localnet_docker_volumes().join("lotus-data");
    let lotus_data_mount = format!("{}:/lotus-data", lotus_data_dir.display());

    // Create a unique container name for this operation
    let container_name = format!("foc-{}-curio-db-setup-{}", run_id, sp_index);

    // Build docker run command with database env vars
    let mut docker_args = vec![
        "run",
        "-d",
        "--name",
        &container_name,
        "--network",
        &pdp_network,
        "-v",
        &bin_mount,
        "-v",
        &lotus_data_mount,
    ];

    // Add environment variables for database connection
    let foc_env = build_foc_contract_env_vars(context)?;
    let db_env = build_db_env_vars(context, sp_index)?;
    let lotus_env = build_lotus_env_vars(context)?;

    for env in &db_env {
        docker_args.push("-e");
        docker_args.push(env);
    }

    for env in &foc_env {
        docker_args.push("-e");
        docker_args.push(env);
    }

    for env in &lotus_env {
        docker_args.push("-e");
        docker_args.push(env);
    }

    // Add image and command
    let bash_cmd = format!(
        "sleep 3 && /usr/local/bin/lotus-bins/curio config new-cluster {}",
        miner_id
    );
    docker_args.extend_from_slice(&["foc-curio", "/bin/bash", "-c", &bash_cmd]);

    let output = Command::new("docker").args(&docker_args).output()?;

    if !output.status.success() {
        // Clean up container on failure
        let _ = docker_command(&["rm", "-f", &container_name]);
        return Err(format!(
            "Failed to create base cluster for miner {}: {}",
            miner_id,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Connect to Lotus network for Lotus daemon access
    let _ = docker_command(&["network", "connect", &lotus_network, &container_name]);

    // Wait for the command to complete
    let output = docker_command(&["wait", &container_name])?;

    // Clean up container
    let _ = docker_command(&["rm", "-f", &container_name]);

    if !output.status.success() {
        return Err(format!(
            "Failed to create base cluster for miner {}: command failed",
            miner_id
        )
        .into());
    }

    // Wait for DB changes to propagate
    thread::sleep(Duration::from_secs(DB_SETUP_WAIT_SECS));

    println!(
        "      {} Base cluster created for miner {}",
        "✓".green(),
        miner_id
    );

    Ok(())
}

/// Create PDP layer configuration.
///
/// Runs: `curio config create --title pdp-only` with PDP layer config in a temporary container
fn create_pdp_layer(context: &StepContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    println!("      {} Creating PDP layer configuration...", "⚙".cyan());

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let pdp_network = pdp_miner_network_name(run_id, sp_index);
    let lotus_network = lotus_network_name(run_id);

    // Get binary directory for volume mount
    let bin_dir = foc_localnet_bin();
    let bin_mount = format!("{}:/usr/local/bin/lotus-bins", bin_dir.display());

    // Get lotus-data directory for volume mount (needed for token and LOTUS_PATH)
    let lotus_data_dir = foc_localnet_docker_volumes().join("lotus-data");
    let lotus_data_mount = format!("{}:/lotus-data", lotus_data_dir.display());

    // Generate PDP layer config with sp_index
    let pdp_config = PDP_LAYER_CONFIG_TEMPLATE.replace("{sp_index}", &sp_index.to_string());

    // Create a unique container name for this operation
    let container_name = format!("foc-{}-curio-pdp-setup-{}", run_id, sp_index);

    // Build docker run command with database env vars
    let mut docker_args = vec![
        "run",
        "-d",
        "--name",
        &container_name,
        "--network",
        &pdp_network,
        "-v",
        &bin_mount,
        "-v",
        &lotus_data_mount,
    ];

    // Add environment variables for database connection
    let foc_env = build_foc_contract_env_vars(context)?;
    let db_env = build_db_env_vars(context, sp_index)?;
    let lotus_env = build_lotus_env_vars(context)?;

    for env in &db_env {
        docker_args.push("-e");
        docker_args.push(env);
    }

    for env in &foc_env {
        docker_args.push("-e");
        docker_args.push(env);
    }

    for env in &lotus_env {
        docker_args.push("-e");
        docker_args.push(env);
    }

    // Add image and command with heredoc for config
    let bash_cmd = format!(
        "sleep 5 && /usr/local/bin/lotus-bins/curio config create --title pdp-only << 'EOF'\n{}\nEOF",
        pdp_config
    );

    docker_args.extend_from_slice(&["foc-curio", "/bin/bash", "-c", &bash_cmd]);

    let output = Command::new("docker").args(&docker_args).output()?;

    if !output.status.success() {
        // Clean up container on failure
        let _ = docker_command(&["rm", "-f", &container_name]);
        return Err(format!(
            "Failed to create PDP layer configuration: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Connect to Lotus network for Lotus daemon access
    let _ = docker_command(&["network", "connect", &lotus_network, &container_name]);

    // Wait for the command to complete
    let output = docker_command(&["wait", &container_name])?;

    // Clean up container
    let _ = docker_command(&["rm", "-f", &container_name]);

    if !output.status.success() {
        return Err("Failed to create PDP layer configuration: command failed".into());
    }

    println!("      {} PDP layer configuration created", "✓".green());

    Ok(())
}
