//! Curio daemon management.
//!
//! Handles creating Docker containers and starting Curio daemon instances.

use super::super::step::StepContext;
use super::constants::{CURIO_LAYERS, CURIO_WEB_RPC_PORT, DAEMON_STARTUP_WAIT_SECS};
use super::db_setup::{build_db_env_vars, build_foc_contract_env_vars, build_lotus_env_vars};
use super::CurioStep;
use crate::commands::start::genesis::constants::PDP_SP_MINER_ID_START;
use crate::docker::network::{lotus_network_name, pdp_miner_network_name};
use crate::docker::{container_exists, stop_and_remove_container};
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis_sectors_pdp_sp,
    foc_localnet_proof_parameters, CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Start Curio daemon for a specific PDP SP.
///
/// Steps:
/// 1. Create curio data directories
/// 2. Build Docker run command with proper volumes and env vars
/// 3. Start container with sleep infinity
/// 4. Run curio daemon in background
/// 5. Wait for API to be ready
/// 6. Store allocated ports in context for later use
pub fn start_curio_daemon(
    context: &mut StepContext,
    _step: &CurioStep,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    println!(
        "    {} Starting Curio daemon for PDP SP {}...",
        "🚀".cyan(),
        sp_index
    );

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = format!("foc-{}-curio-{}", run_id, sp_index);

    // Allocate ports dynamically for this Curio instance
    let api_port = context.port_allocator.allocate()?;
    let api_port_alt = context.port_allocator.allocate()?;
    let gui_port = context.port_allocator.allocate()?;
    let pdp_port = context.port_allocator.allocate()?;

    // Store allocated ports in context for later use (e.g., registration step)
    context.set(
        format!("curio_sp_{}_api_port", sp_index),
        api_port.to_string(),
    );
    context.set(
        format!("curio_sp_{}_api_port_alt", sp_index),
        api_port_alt.to_string(),
    );
    context.set(
        format!("curio_sp_{}_gui_port", sp_index),
        gui_port.to_string(),
    );
    context.set(
        format!("curio_sp_{}_pdp_port", sp_index),
        pdp_port.to_string(),
    );

    // Clean up existing container if any
    if container_exists(&container_name)? {
        println!(
            "      {} Removing existing container {}...",
            "🗑️".yellow(),
            container_name
        );
        stop_and_remove_container(&container_name)?;
    }

    // Create necessary directories
    create_curio_directories(sp_index)?;

    // Create and start container with curio as main process
    create_curio_container(context, sp_index, &container_name)?;

    // Wait for daemon to be ready
    wait_for_daemon_ready(&container_name)?;

    println!(
        "    {} Curio daemon started for PDP SP {}",
        "✓".green(),
        sp_index
    );

    Ok(())
}

/// Create necessary directories for Curio
fn create_curio_directories(sp_index: usize) -> Result<(), Box<dyn Error>> {
    let volumes_dir = foc_localnet_docker_volumes();
    let curio_sp_dir = volumes_dir.join("curio").join(sp_index.to_string());

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
fn create_curio_container(
    context: &StepContext,
    sp_index: usize,
    container_name: &str,
) -> Result<(), Box<dyn Error>> {
    println!(
        "      {} Creating container {}...",
        "🐳".cyan(),
        container_name
    );

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let miner_id = format!("t0{}", PDP_SP_MINER_ID_START + (sp_index as u32) - 1);

    // Build docker run command
    let mut docker_args = build_docker_run_args(context, sp_index, container_name, &miner_id)?;

    // Add image and command - run curio directly as the main process
    docker_args.push("foc-curio".to_string());
    docker_args.push("/usr/local/bin/lotus-bins/curio".to_string());
    docker_args.push("run".to_string());
    docker_args.push("--nosync".to_string());
    docker_args.push("--layers".to_string());
    docker_args.push(CURIO_LAYERS.to_string());

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
    let lotus_network = lotus_network_name(run_id);
    let _ = Command::new("docker")
        .args(["network", "connect", &lotus_network, container_name])
        .output(); // Ignore errors if already connected

    println!("      {} Container created", "✓".green());

    Ok(())
}

/// Build Docker run arguments for Curio container
fn build_docker_run_args(
    context: &StepContext,
    sp_index: usize,
    container_name: &str,
    _miner_id: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let pdp_network = pdp_miner_network_name(run_id, sp_index);

    let volumes_dir = foc_localnet_docker_volumes();
    let curio_sp_dir = volumes_dir.join("curio").join(sp_index.to_string());
    let lotus_data_dir = volumes_dir.join("lotus-data");
    let sectors_dir = foc_localnet_genesis_sectors_pdp_sp(sp_index);
    let bin_dir = foc_localnet_bin();
    let params_dir = foc_localnet_proof_parameters();

    let mut docker_args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.to_string(),
        "--network".to_string(),
        pdp_network,
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
            "{}:/home/foc-user/.lotus-local-net",
            lotus_data_dir.display()
        ),
        format!("{}:/lotus-data:ro", lotus_data_dir.display()),
        format!("{}:/sectors", sectors_dir.display()),
        format!(
            "{}:{}",
            params_dir.display(),
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

/// Wait for Curio daemon to be ready
fn wait_for_daemon_ready(container_name: &str) -> Result<(), Box<dyn Error>> {
    println!("      {} Waiting for daemon to be ready...", "⏳".cyan());

    thread::sleep(Duration::from_secs(DAEMON_STARTUP_WAIT_SECS));

    // Check if API is responding
    for attempt in 1..=12 {
        let output = Command::new("docker")
            .args([
                "exec",
                container_name,
                "curl",
                "-s",
                &format!("http://localhost:{}/api/webrpc/v0", CURIO_WEB_RPC_PORT),
            ])
            .output()?;

        if output.status.success() {
            println!("      {} Daemon is ready", "✓".green());
            return Ok(());
        }

        if attempt < 12 {
            println!(
                "      {} Waiting for API (attempt {}/12)...",
                "⏳".dim(),
                attempt
            );
            thread::sleep(Duration::from_secs(5));
        }
    }

    Err("Curio daemon did not become ready within timeout".into())
}
