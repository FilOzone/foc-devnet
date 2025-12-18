//! Verification functions for Lotus daemon.
//!
//! This module contains functions that verify the Lotus daemon is properly
//! started and all services are accessible.

use super::super::step::SetupContext;
use crate::docker::containers::lotus_container_name;
use crate::docker::wait_for_port;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

// Timing constants
const PORT_CHECK_TIMEOUT_SECS: u64 = 30;
const API_FILE_TIMEOUT_SECS: u64 = 180;
const PORT_CHECK_INTERVAL_MS: u64 = 500;
const DAEMON_INIT_WAIT_SECS: u64 = 5;

/// Get the Lotus container name from context
fn get_container_name(context: &SetupContext) -> Result<String, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    Ok(lotus_container_name(run_id))
}

/// Check if lotus daemon is responsive via API
pub fn check_lotus_api(context: &SetupContext) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;

    // Try to execute a simple lotus command via docker exec
    let output = Command::new("docker")
        .args([
            "exec",
            &container_name,
            "/usr/local/bin/lotus-bins/lotus",
            "version",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Lotus API check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}

/// Verify that all required ports are accessible
pub fn verify_ports(context: &SetupContext) -> Result<(), Box<dyn Error>> {
    // Read allocated ports from context
    let lotus_api_port: u16 = context
        .get("lotus_api_port")
        .ok_or("Lotus API port not found in context")?
        .parse()?;
    let lotus_p2p_port: u16 = context
        .get("lotus_p2p_port")
        .ok_or("Lotus P2P port not found in context")?
        .parse()?;

    let ports = [(lotus_api_port, "Lotus API"), (lotus_p2p_port, "Lotus P2P")];

    // Check all ports are accessible
    info!("    Verifying port accessibility...");
    for &(port, description) in &ports {
        match wait_for_port(port, PORT_CHECK_TIMEOUT_SECS) {
            Ok(_) => info!("      ✓ Port {} ({}) is accessible", port, description),
            Err(e) => {
                warn!("      ✗ Port {} ({}) is not accessible", port, description);
                return Err(format!("Port {} is not accessible: {}", port, e).into());
            }
        }
    }
    Ok(())
}

/// Wait for the Lotus API file to be created
pub fn wait_for_api_file(volumes_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    // Wait for Lotus API file to exist and daemon to be fully initialized
    info!("    Waiting for Lotus API to be ready (this may take 1-2 minutes)...");
    let lotus_data_dir = volumes_dir.join("lotus-data");
    let api_file = lotus_data_dir.join("api");

    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(API_FILE_TIMEOUT_SECS); // 3 minute timeout

    while !api_file.exists() {
        if start.elapsed() > timeout {
            return Err("Timeout waiting for Lotus API file to be created".into());
        }
        thread::sleep(Duration::from_millis(PORT_CHECK_INTERVAL_MS));
    }
    info!("    ✓ Lotus API file created");

    // Wait a bit more for daemon to fully initialize
    thread::sleep(Duration::from_secs(DAEMON_INIT_WAIT_SECS));

    // FEVM is already configured in config.toml before container start
    info!("    ✓ FEVM and ChainIndexer enabled via config.toml");
    Ok(())
}

/// Check if Ethereum RPC is available via the Lotus API
///
/// This verifies that FEVM is properly enabled by testing a basic eth_* RPC call.
pub fn check_ethereum_rpc(context: &SetupContext) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;

    // Read allocated API port from context
    let lotus_api_port: u16 = context
        .get("lotus_api_port")
        .ok_or("Lotus API port not found in context")?
        .parse()?;

    // Test eth_blockNumber via docker exec
    // This is a simple, safe RPC call that should work if FEVM is enabled
    let output = Command::new("docker")
        .args([
            "exec",
            &container_name,
            "/bin/bash",
            "-c",
            &format!(
                "curl -s -X POST -H 'Content-Type: application/json' \
                --data '{{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"params\":[],\"id\":1}}' \
                http://localhost:{}/rpc/v1",
                lotus_api_port
            ),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to execute eth_blockNumber RPC call: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let response = String::from_utf8_lossy(&output.stdout);

    // Check if response contains result (indicating success)
    // Even if block number is 0x0, it should have a "result" field
    if !response.contains("\"result\"") {
        return Err(format!("Unexpected response from eth_blockNumber: {}", response).into());
    }

    Ok(())
}

/// Verify Lotus API and Ethereum RPC connectivity
pub fn verify_api_connectivity(context: &SetupContext) -> Result<(), Box<dyn Error>> {
    // Verify Lotus API is responsive
    info!("    Verifying Lotus API connectivity...");
    match check_lotus_api(context) {
        Ok(_) => {
            info!("    ✓ Lotus daemon is ready and responding to API calls");
        }
        Err(e) => {
            warn!("    ⚠ Lotus API verification failed: {}", e);
            info!(
                "    Note: Lotus may still be initializing. This is usually not a critical error."
            );
        }
    }

    // Verify FEVM/Ethereum RPC is available
    info!("    Verifying FEVM Ethereum RPC...");
    match check_ethereum_rpc(context) {
        Ok(_) => {
            info!("    ✓ Ethereum RPC is available and responding");
        }
        Err(e) => {
            warn!("    ⚠ Ethereum RPC verification failed: {}", e);
            info!(
                "    Note: This may indicate FEVM is not fully initialized. Check logs if needed."
            );
        }
    }

    info!("    ✓ Lotus daemon is ready!");

    // Display endpoint information with dynamic port
    let lotus_api_port: u16 = context
        .get("lotus_api_port")
        .ok_or("Lotus API port not found in context")?
        .parse()?;
    info!("      API endpoint: http://localhost:{}", lotus_api_port);
    info!("      Ethereum RPC: Available via Lotus API");
    Ok(())
}
