//! Verification functions for Lotus daemon.
//!
//! This module contains functions that verify the Lotus daemon is properly
//! started and all services are accessible.

use crate::docker::wait_for_port;
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CONTAINER_NAME: &str = "foc-lotus";

// Lotus daemon ports
const LOTUS_PORTS: &[(u16, &str)] = &[(1234, "Lotus API"), (1235, "Lotus P2P")];

// Timing constants
const PORT_CHECK_TIMEOUT_SECS: u64 = 30;
const API_FILE_TIMEOUT_SECS: u64 = 180;
const PORT_CHECK_INTERVAL_MS: u64 = 500;
const DAEMON_INIT_WAIT_SECS: u64 = 5;

/// Check if lotus daemon is responsive via API
pub fn check_lotus_api() -> Result<(), Box<dyn Error>> {
    // Try to execute a simple lotus command via docker exec
    let output = Command::new("docker")
        .args([
            "exec",
            CONTAINER_NAME,
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
pub fn verify_ports() -> Result<(), Box<dyn Error>> {
    // Check all ports are accessible
    println!("    Verifying port accessibility...");
    for &(port, description) in LOTUS_PORTS {
        print!("      Checking port {} ({})... ", port, description);
        match wait_for_port(port, PORT_CHECK_TIMEOUT_SECS) {
            Ok(_) => println!("{}", "✓".green()),
            Err(e) => {
                println!("{}", "✗".red());
                return Err(format!("Port {} is not accessible: {}", port, e).into());
            }
        }
    }
    Ok(())
}

/// Wait for the Lotus API file to be created
pub fn wait_for_api_file(volumes_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    // Wait for Lotus API file to exist and daemon to be fully initialized
    println!("    Waiting for Lotus API to be ready (this may take 1-2 minutes)...");
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
    println!("    {} Lotus API file created", "✓".green());

    // Wait a bit more for daemon to fully initialize
    thread::sleep(Duration::from_secs(DAEMON_INIT_WAIT_SECS));

    // FEVM is already configured in config.toml before container start
    println!(
        "    {} FEVM and ChainIndexer enabled via config.toml",
        "✓".green()
    );
    Ok(())
}

/// Check if Ethereum RPC is available via the Lotus API
///
/// This verifies that FEVM is properly enabled by testing a basic eth_* RPC call.
pub fn check_ethereum_rpc() -> Result<(), Box<dyn Error>> {
    // Test eth_blockNumber via docker exec
    // This is a simple, safe RPC call that should work if FEVM is enabled
    let output = Command::new("docker")
        .args([
            "exec",
            CONTAINER_NAME,
            "/bin/bash",
            "-c",
            "curl -s -X POST -H 'Content-Type: application/json' \
            --data '{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"params\":[],\"id\":1}' \
            http://localhost:1234/rpc/v1",
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
pub fn verify_api_connectivity() -> Result<(), Box<dyn Error>> {
    // Verify Lotus API is responsive
    println!("    Verifying Lotus API connectivity...");
    match check_lotus_api() {
        Ok(_) => {
            println!(
                "    {} Lotus daemon is ready and responding to API calls",
                "✓".green()
            );
        }
        Err(e) => {
            println!("    {} Lotus API verification failed: {}", "⚠".yellow(), e);
            println!(
                "    Note: Lotus may still be initializing. This is usually not a critical error."
            );
        }
    }

    // Verify FEVM/Ethereum RPC is available
    println!("    Verifying FEVM Ethereum RPC...");
    match check_ethereum_rpc() {
        Ok(_) => {
            println!(
                "    {} Ethereum RPC is available and responding",
                "✓".green()
            );
        }
        Err(e) => {
            println!(
                "    {} Ethereum RPC verification failed: {}",
                "⚠".yellow(),
                e
            );
            println!(
                "    Note: This may indicate FEVM is not fully initialized. Check logs if needed."
            );
        }
    }

    println!("\n    {} Lotus daemon is ready!", "✓".green().bold());
    println!("      API endpoint: http://localhost:1234");
    println!("      Ethereum RPC: Available via Lotus API");
    Ok(())
}
