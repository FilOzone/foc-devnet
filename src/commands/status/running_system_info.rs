//! Running system information display for foc-localnet.
//!
//! This module displays detailed information about the currently running
//! system, including block height, service ports, and file locations.

use crate::paths::{contract_addresses_file, foc_metadata_file, step_context_file};
use crate::run_id::load_current_run_id;
use std::process::Command;
use tracing::{info, warn};

/// Print running system information.
///
/// This displays detailed info when the system is running, including:
/// - Current run ID
/// - Block height
/// - Service ports
/// - File locations for addresses and context dumps
pub fn print_running_system_info() -> Result<(), Box<dyn std::error::Error>> {
    // Try to load current run ID
    let run_id = match load_current_run_id() {
        Ok(id) => id,
        Err(_) => {
            // No run ID means system is not running
            return Ok(());
        }
    };

    info!("Current Run ID: {}", run_id);

    // Get block height
    if let Some(height) = get_lotus_block_height(&run_id) {
        info!("Chain Block Height: {}", height);
    } else {
        warn!("Chain Block Height: Unable to retrieve");
    }

    // Print service ports
    print_service_ports(&run_id)?;

    // Print file locations
    print_file_locations(&run_id)?;

    Ok(())
}

/// Get the current lotus chain block height.
///
/// This function queries the lotus node to get the current block height of the chain.
fn get_lotus_block_height(run_id: &str) -> Option<u64> {
    let container_name = format!("foc-{}-lotus", run_id);

    let output = Command::new("docker")
        .args([
            "exec",
            &container_name,
            "/usr/local/bin/lotus-bins/lotus",
            "chain",
            "list",
            "--count=1",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the block height from the first line (format: "HEIGHT: (timestamp) [ ... ]")
    for line in stdout.lines() {
        if let Some(colon_pos) = line.find(':') {
            let height_str = line[..colon_pos].trim();
            if let Ok(height) = height_str.parse::<u64>() {
                return Some(height);
            }
        }
    }

    None
}

/// Print service ports for accessing various services.
fn print_service_ports(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Get port for each service
    let services = vec![
        ("Lotus RPC", format!("foc-{}-lotus", run_id), "1234/tcp"),
        ("Lotus P2P", format!("foc-{}-lotus", run_id), "1235/tcp"),
        (
            "Lotus Miner API",
            format!("foc-{}-lotus-miner", run_id),
            "2345/tcp",
        ),
    ];

    for (service_name, container_name, internal_port) in services {
        if let Ok(port) = get_container_port(&container_name, internal_port) {
            info!("{}: http://0.0.0.0:{}", service_name, port);
        }
    }

    // Check for Curio instances
    for sp_idx in 1..=5 {
        let curio_container = format!("foc-{}-curio-{}", run_id, sp_idx);
        if container_exists(&curio_container) {
            if let Ok(port) = get_container_port(&curio_container, "12300/tcp") {
                info!("Curio SP-{} API: http://0.0.0.0:{}", sp_idx, port);
            }
        }
    }

    Ok(())
}

/// Check if a container exists.
fn container_exists(container_name: &str) -> bool {
    Command::new("docker")
        .args(["inspect", container_name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the mapped port for a container's internal port.
fn get_container_port(
    container_name: &str,
    internal_port: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["port", container_name, internal_port])
        .output()?;

    if !output.status.success() {
        return Err("Failed to get port".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse output like "0.0.0.0:1234"
    if let Some(colon_pos) = stdout.rfind(':') {
        let port = stdout[colon_pos + 1..].trim();
        Ok(port.to_string())
    } else {
        Err("Port not mapped".into())
    }
}

/// Print file locations for addresses and context dumps.
fn print_file_locations(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let contract_addr_file = contract_addresses_file(run_id);
    info!("Deployed Addresses: {}", contract_addr_file.display());

    let foc_meta_file = foc_metadata_file(run_id);
    info!("FOC Metadata: {}", foc_meta_file.display());

    let step_ctx_file = step_context_file(run_id);
    info!("Step Context: {}", step_ctx_file.display());

    Ok(())
}
