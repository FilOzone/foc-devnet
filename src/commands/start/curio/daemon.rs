//! Curio daemon management.
//!
//! Handles creating Docker containers and starting Curio daemon instances.

use super::super::step::SetupContext;
use super::db_setup::{build_db_env_vars, build_foc_contract_env_vars, build_lotus_env_vars};
use super::CurioStep;
use crate::docker::network::{lotus_network_name, pdp_miner_network_name};
use crate::docker::{container_exists, stop_and_remove_container};
use crate::paths::{
    foc_localnet_bin, foc_localnet_curio_sp_volume, foc_localnet_genesis_sectors_pdp_sp,
    foc_localnet_proof_parameters, CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
};
use std::error::Error;
use std::fs;
use std::process::Command;
use tracing::info;

/// Start a single Curio PDP SP daemon.
///
/// Steps:
/// 1. Create curio data directories
/// 2. Build Docker run command with proper volumes and env vars
/// 3. Start container with sleep infinity
/// 4. Run curio daemon in background
/// 5. Wait for API to be ready
/// 6. Store allocated ports in context for later use
pub fn start_curio_daemon(
    context: &SetupContext,
    _step: &CurioStep,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    info!("Starting Curio daemon for PDP SP {}...", sp_index);

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = format!("foc-{}-curio-{}", run_id, sp_index);

    // Clean up existing container if any
    if container_exists(&container_name)? {
        info!("Removing existing container {}...", container_name);
        stop_and_remove_container(&container_name)?;
    }

    // Create necessary directories
    create_curio_directories(context, sp_index)?;

    // Step 2: Create and start container
    let docker_args = build_docker_run_args(context, sp_index, &container_name)?;
    start_curio_container(context, &container_name, docker_args)?;

    Ok(())
}

/// Create necessary directories for Curio
fn create_curio_directories(context: &SetupContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let curio_sp_dir = foc_localnet_curio_sp_volume(run_id, sp_index);

    let dirs = vec![
        curio_sp_dir.join(".curio"),
        curio_sp_dir.join("fast-storage"),
        curio_sp_dir.join("long-term-storage"),
    ];

    for dir in dirs {
        fs::create_dir_all(&dir)?;
    }

    Ok(())
}

/// Create and start Curio container
fn start_curio_container(
    context: &SetupContext,
    container_name: &str,
    docker_args: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    info!("Creating container {}...", container_name);

    // Execute docker run
    let output = Command::new("docker").args(&docker_args).output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create curio container: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Connect to filecoin network
    let lotus_network = lotus_network_name(context.run_id().ok_or("Run ID not found in context")?);
    let _ = Command::new("docker")
        .args(["network", "connect", &lotus_network, container_name])
        .output(); // Ignore errors if already connected

    info!("Container created");

    Ok(())
}

/// Build docker run arguments for Curio
fn build_docker_run_args(
    context: &SetupContext,
    sp_index: usize,
    container_name: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let curio_sp_dir = foc_localnet_curio_sp_volume(run_id, sp_index);
    let bin_dir = foc_localnet_bin();
    let proof_params_dir = foc_localnet_proof_parameters();
    let genesis_sectors_dir = foc_localnet_genesis_sectors_pdp_sp(run_id, sp_index);

    let mut docker_args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.to_string(),
        "--network".to_string(),
        pdp_miner_network_name(run_id, sp_index),
    ];

    // Port mappings - get dynamically allocated ports from context
    let api_port: u16 = context
        .get(&format!("curio_sp_{}_api_port", sp_index))
        .ok_or("Curio API port not found in context")?
        .parse()?;
    let api_port_alt: u16 = context
        .get(&format!("curio_sp_{}_api_port_alt", sp_index))
        .ok_or("Curio API alt port not found in context")?
        .parse()?;
    let gui_port: u16 = context
        .get(&format!("curio_sp_{}_gui_port", sp_index))
        .ok_or("Curio GUI port not found in context")?
        .parse()?;
    let pdp_port: u16 = context
        .get(&format!("curio_sp_{}_pdp_port", sp_index))
        .ok_or("Curio PDP port not found in context")?
        .parse()?;

    docker_args.extend_from_slice(&[
        "-p".to_string(),
        format!("{}:12300", api_port),
        "-p".to_string(),
        format!("{}:12301", api_port_alt),
        "-p".to_string(),
        format!("{}:4701", gui_port),
        "-p".to_string(),
        format!("{}:4702", pdp_port),
    ]);

    // Volume mounts
    let volume_mounts = vec![
        format!(
            "{}:/home/foc-user/.curio",
            curio_sp_dir.join(".curio").display()
        ),
        format!(
            "{}:/home/foc-user/curio/fast-storage",
            curio_sp_dir.join("fast-storage").display()
        ),
        format!(
            "{}:/home/foc-user/curio/long-term-storage",
            curio_sp_dir.join("long-term-storage").display()
        ),
        format!("{}:/usr/local/bin/lotus-bins", bin_dir.display()),
        format!(
            "{}:/home/foc-user/genesis-sectors:ro",
            genesis_sectors_dir.display()
        ),
        format!(
            "{}:{}",
            proof_params_dir.display(),
            CONTAINER_FILECOIN_PROOF_PARAMS_PATH
        ),
    ];

    for mount in volume_mounts {
        docker_args.extend_from_slice(&["-v".to_string(), mount]);
    }

    // Add environment variables using shared builders
    let foc_env = build_foc_contract_env_vars(context)?;
    let db_env = build_db_env_vars(context, sp_index)?;
    let lotus_env = build_lotus_env_vars(context)?;

    for env in &foc_env {
        docker_args.extend_from_slice(&["-e".to_string(), env.clone()]);
    }

    for env in &db_env {
        docker_args.extend_from_slice(&["-e".to_string(), env.clone()]);
    }

    for env in &lotus_env {
        docker_args.extend_from_slice(&["-e".to_string(), env.clone()]);
    }

    Ok(docker_args)
}
