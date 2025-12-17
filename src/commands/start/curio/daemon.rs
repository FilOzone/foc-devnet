//! Curio daemon management.
//!
//! Handles creating Docker containers and starting Curio daemon instances.

use super::super::env_vars::build_network_env_vars;
use super::super::step::StepContext;
use super::CurioStep;
use super::constants::{CURIO_LAYERS, CURIO_WEB_RPC_PORT, DAEMON_STARTUP_WAIT_SECS};
use crate::commands::start::genesis::constants::PDP_SP_MINER_ID_START;
use crate::constants::CURIO_CONTAINER;
use crate::docker::network::{curio_miner_network_name, lotus_network_name};
use crate::docker::{container_exists, stop_and_remove_container};
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis_sectors_pdp_sp,
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
pub fn start_curio_daemon(
    context: &StepContext,
    _step: &CurioStep,
    sp_index: usize,
) -> Result<(), Box<dyn Error>> {
    println!(
        "    {} Starting Curio daemon for PDP SP {}...",
        "🚀".cyan(),
        sp_index
    );

    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = format!("{}-{}-{}", CURIO_CONTAINER, sp_index, run_id);

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

    // Create and start container
    create_curio_container(context, sp_index, &container_name)?;

    // Start curio daemon process inside container
    start_daemon_process(&container_name)?;

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

    // Add image and command
    docker_args.push("foc-curio".to_string());
    docker_args.push("sleep".to_string());
    docker_args.push("infinity".to_string());

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
    let pdp_network = curio_miner_network_name(run_id);
    let yugabyte_name = format!("foc-yugabyte-{}-{}", sp_index, run_id);
    let lotus_name = format!("foc-lotus-{}", run_id);

    let volumes_dir = foc_localnet_docker_volumes();
    let curio_sp_dir = volumes_dir.join("curio").join(sp_index.to_string());
    let lotus_data_dir = volumes_dir.join("lotus-data");
    let sectors_dir = foc_localnet_genesis_sectors_pdp_sp(sp_index);
    let bin_dir = foc_localnet_bin();

    let mut docker_args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.to_string(),
        "--network".to_string(),
        pdp_network,
    ];

    // Port mappings - each SP gets unique ports
    let base_api_port = 12300 + ((sp_index - 1) * 10) as u16;
    let base_gui_port = 4700 + ((sp_index - 1) * 10) as u16;

    docker_args.extend_from_slice(&[
        "-p".to_string(),
        format!("{}:12300", base_api_port),
        "-p".to_string(),
        format!("{}:12301", base_api_port + 1),
        "-p".to_string(),
        format!("{}:4701", base_gui_port + 1),
        "-p".to_string(),
        format!("{}:4702", base_gui_port + 2),
    ]);

    // Volume mounts
    let volume_mounts = vec![
        format!("{}:/home/foc-user/.curio", curio_sp_dir.join(".curio").display()),
        format!(
            "{}:/home/foc-user/curio/fast-storage",
            curio_sp_dir.join("fast-storage").display()
        ),
        format!(
            "{}:/home/foc-user/curio/long-term-storage",
            curio_sp_dir.join("long-term-storage").display()
        ),
        format!("{}:/usr/local/bin/lotus-bins", bin_dir.display()),
        format!("{}:/home/foc-user/.lotus-local-net", lotus_data_dir.display()),
        format!("{}:/sectors", sectors_dir.display()),
    ];

    for mount in volume_mounts {
        docker_args.extend_from_slice(&["-v".to_string(), mount]);
    }

    // Environment variables
    docker_args.extend(build_network_env_vars());

    // Yugabyte DB configuration
    docker_args.extend_from_slice(&[
        "-e".to_string(),
        format!("CURIO_DB_HOST={}", yugabyte_name),
        "-e".to_string(),
        "CURIO_DB_PORT=5433".to_string(),
        "-e".to_string(),
        "CURIO_DB_USER=yugabyte".to_string(),
        "-e".to_string(),
        "CURIO_DB_PASSWORD=yugabyte".to_string(),
        "-e".to_string(),
        "CURIO_DB_NAME=yugabyte".to_string(),
        "-e".to_string(),
        "CURIO_DB_LOAD_BALANCE=false".to_string(),
    ]);

    // Lotus API configuration
    let lotus_api = format!("http://{}:1234/rpc/v1", lotus_name);
    docker_args.extend_from_slice(&[
        "-e".to_string(),
        format!("LOTUS_API={}", lotus_api),
        "-e".to_string(),
        "LOTUS_PATH=/home/foc-user/.lotus-local-net".to_string(),
    ]);

    Ok(docker_args)
}

/// Start curio daemon process inside container
fn start_daemon_process(container_name: &str) -> Result<(), Box<dyn Error>> {
    println!(
        "      {} Starting daemon with layers: {}...",
        "⚙".cyan(),
        CURIO_LAYERS
    );

    let output = Command::new("docker")
        .args([
            "exec",
            "-d",
            container_name,
            "/usr/local/bin/lotus-bins/curio",
            "run",
            "--nosync",
            "--layers",
            CURIO_LAYERS,
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to start curio daemon: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
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
