//! Curio daemon management.
//!
//! Handles creating Docker containers and starting Curio daemon instances.

use super::super::step::SetupContext;
use super::db_setup::{build_db_env_vars, build_foc_contract_env_vars, build_lotus_env_vars};
use super::CurioStep;
use crate::commands::start::curio::constants::CURIO_LAYERS;
use crate::docker::command_logger::run_and_log_command_strings;
use crate::docker::init::set_volume_ownership;
use crate::docker::network::{lotus_network_name, pdp_miner_network_name};
use crate::docker::{container_exists, stop_and_remove_container};
use crate::paths::{
    foc_devnet_bin, foc_devnet_curio_sp_volume, foc_devnet_genesis_sectors_pdp_sp,
    foc_devnet_proof_parameters, CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
};
use std::error::Error;
use std::fs;
use tracing::info;

/// Start a single Curio PDP SP daemon.
///
/// Steps:
/// 1. Create curio data directories
/// 2. Build Docker run command with proper volumes and env vars
/// 3. Start container with sleep infinity
/// 4. Run curio daemon in background
/// 5. Store container ID and name in context for later use
pub fn start_curio_daemon(
    context: &SetupContext,
    _step: &CurioStep,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    info!("Starting Curio daemon for PDP SP {}...", sp_index);

    let run_id = context.run_id();
    let container_name = format!("foc-{}-curio-{}", run_id, sp_index);

    // Clean up existing container if any
    if container_exists(&container_name)? {
        info!("Removing existing container {}...", container_name);
        stop_and_remove_container(&container_name)?;
    }

    // Create necessary directories
    create_curio_directories(context, sp_index)?;

    // Step 2: Create and start container, capturing container ID
    let docker_args = build_docker_create_args(context, sp_index, &container_name)?;
    let container_id = start_curio_container(context, &container_name, docker_args)?;

    // Store container info in context for export
    context.set(format!("pdp_sp_{}_container_id", sp_index), container_id);
    context.set(
        format!("pdp_sp_{}_container_name", sp_index),
        container_name,
    );

    Ok(())
}

/// Create necessary directories for Curio
fn create_curio_directories(context: &SetupContext, sp_index: usize) -> Result<(), Box<dyn Error>> {
    let run_id = context.run_id();
    let curio_sp_dir = foc_devnet_curio_sp_volume(run_id, sp_index);

    let dirs = vec![
        curio_sp_dir.join(".curio"),
        curio_sp_dir.join("fast-storage"),
        curio_sp_dir.join("long-term-storage"),
    ];

    for dir in &dirs {
        fs::create_dir_all(dir)?;
    }

    set_volume_ownership(&curio_sp_dir)?;

    Ok(())
}

/// Create and start Curio container
///
/// Uses docker create + network connect + start pattern so that:
/// 1. Container is created but not started
/// 2. Networks are connected while container is stopped
/// 3. Container is started with Curio as PID 1 (logs work properly)
///
/// Returns the container ID from docker create stdout
fn start_curio_container(
    context: &SetupContext,
    container_name: &str,
    mut docker_args: Vec<String>,
) -> Result<String, Box<dyn Error>> {
    info!("Creating container {}...", container_name);

    // Add image and command - Curio as main process
    docker_args.push(crate::constants::CURIO_DOCKER_IMAGE.to_string());
    docker_args.push("/usr/local/bin/lotus-bins/curio".to_string());
    docker_args.push("run".to_string());
    docker_args.push("--nosync".to_string());
    docker_args.push("--layers".to_string());
    docker_args.push(CURIO_LAYERS.to_string());

    // Execute docker create (not run)
    let key = format!("curio_daemon_create_{}", container_name);
    let output = run_and_log_command_strings("docker", &docker_args, context, &key)?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create curio container: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    // Extract container ID from stdout
    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if container_id.is_empty() {
        return Err("Docker create did not return container ID".into());
    }

    // Connect to filecoin network before starting
    let lotus_network = lotus_network_name(context.run_id());
    let network_args = vec![
        "network".to_string(),
        "connect".to_string(),
        lotus_network.clone(),
        container_name.to_string(),
    ];
    let key = format!("curio_network_connect_{}", container_name);
    let _ = run_and_log_command_strings("docker", &network_args, context, &key); // Ignore errors if already connected

    // Start the container
    let start_args = vec!["start".to_string(), container_name.to_string()];
    let key = format!("curio_daemon_start_{}", container_name);
    let output = run_and_log_command_strings("docker", &start_args, context, &key)?;

    if !output.status.success() {
        return Err(format!(
            "Failed to start curio container: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    info!("Container created and started");

    Ok(container_id)
}

/// Build docker create arguments for Curio
fn build_docker_create_args(
    context: &SetupContext,
    sp_index: usize,
    container_name: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let run_id = context.run_id();
    let curio_sp_dir = foc_devnet_curio_sp_volume(run_id, sp_index);
    let bin_dir = foc_devnet_bin();
    let proof_params_dir = foc_devnet_proof_parameters();
    let genesis_sectors_dir = foc_devnet_genesis_sectors_pdp_sp(run_id, sp_index);

    let mut docker_args = vec![
        "create".to_string(),
        "--name".to_string(),
        container_name.to_string(),
        "--network".to_string(),
        pdp_miner_network_name(run_id, sp_index),
        // Enable host.docker.internal for SP-to-SP fetch (resolves to host gateway)
        "--add-host=host.docker.internal:host-gateway".to_string(),
    ];

    // Port mappings - get dynamically allocated ports from context
    let api_port: u16 = context
        .get(&format!("pdp_sp_{}_api_port", sp_index))
        .ok_or("Curio API port not found in context")?
        .parse()?;
    let api_port_alt: u16 = context
        .get(&format!("pdp_sp_{}_api_port_alt", sp_index))
        .ok_or("Curio API alt port not found in context")?
        .parse()?;
    let gui_port: u16 = context
        .get(&format!("pdp_sp_{}_gui_port", sp_index))
        .ok_or("Curio GUI port not found in context")?
        .parse()?;
    let pdp_port: u16 = context
        .get(&format!("pdp_sp_{}_pdp_port", sp_index))
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

    docker_args.push("-e".to_string());
    docker_args.push("GOLOG_LOG_LEVEL=pdp=debug".to_string());

    Ok(docker_args)
}
