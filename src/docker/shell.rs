//! High-level shell command abstractions.
//!
//! This module provides high-level abstractions for blockchain-related shell commands
//! like Lotus, Forge, Cast, and other tools used in foc-localnet.

use crate::docker::core::{docker_command, exec_in_container};
use std::error::Error;
use std::process::Output;

/// Execute a lotus command inside the foc-lotus container.
pub fn lotus_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    exec_in_container("foc-lotus", "/usr/local/bin/lotus-bins/lotus", args)
}

/// Execute a lotus-miner command inside the foc-lotus-miner container.
pub fn lotus_miner_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    exec_in_container(
        "foc-lotus-miner",
        "/usr/local/bin/lotus-bins/lotus-miner",
        args,
    )
}

/// Execute a forge command inside the foc-builder container.
pub fn forge_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    exec_in_container("foc-builder", "forge", args)
}

/// Execute a cast command inside the foc-builder container.
pub fn cast_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    exec_in_container("foc-builder", "cast", args)
}

/// Execute a lotus wallet command.
pub fn lotus_wallet_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut full_args = vec!["wallet"];
    full_args.extend_from_slice(args);
    lotus_command(&full_args)
}

/// Execute a lotus evm command.
pub fn lotus_evm_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut full_args = vec!["evm"];
    full_args.extend_from_slice(args);
    lotus_command(&full_args)
}

/// Execute a lotus send command to transfer FIL.
pub fn lotus_send_fil(from: &str, to: &str, amount: &str) -> Result<Output, Box<dyn Error>> {
    lotus_command(&["send", "--from", from, to, amount])
}

/// Create a new delegated (f4) address for FEVM operations.
pub fn lotus_create_delegated_address() -> Result<String, Box<dyn Error>> {
    let output = lotus_wallet_command(&["new", "delegated"])?;
    let address = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(address)
}

/// Import a key into lotus wallet from a hex-encoded keyinfo file.
pub fn lotus_import_key(keyinfo_path: &str) -> Result<String, Box<dyn Error>> {
    let output = lotus_wallet_command(&["import", keyinfo_path])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let address = stdout
        .lines()
        .find(|line| line.starts_with("imported key"))
        .and_then(|line| line.split_whitespace().nth(2))
        .ok_or("Failed to extract imported address")?
        .to_string();
    Ok(address)
}

/// Export a private key from lotus wallet.
pub fn lotus_export_key(address: &str) -> Result<Output, Box<dyn Error>> {
    lotus_wallet_command(&["export", address])
}

/// Get the Ethereum address corresponding to an f4 address.
pub fn lotus_get_eth_address(f4_address: &str) -> Result<String, Box<dyn Error>> {
    let output = lotus_evm_command(&["stat", f4_address])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let eth_addr = stdout
        .lines()
        .find(|line| line.contains("Eth address:"))
        .and_then(|line| line.split_whitespace().nth(2))
        .ok_or("Failed to extract Ethereum address")?
        .to_string();
    Ok(eth_addr)
}

/// Run a docker container with host networking.
pub fn docker_run_host_network(image: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut full_args = vec!["run", "--rm", "--network", "host"];
    full_args.extend_from_slice(args);
    full_args.extend_from_slice(&["-i", image]);
    docker_command(&full_args)
}

/// Run a docker container with volume mounts.
pub fn docker_run_with_volumes(
    image: &str,
    volumes: &[&str],
    args: &[&str],
) -> Result<Output, Box<dyn Error>> {
    let mut full_args = vec!["run", "--rm"];
    for volume in volumes {
        full_args.push("-v");
        full_args.push(volume);
    }
    full_args.extend_from_slice(args);
    full_args.push(image);
    docker_command(&full_args)
}

/// Execute a bash command inside the foc-builder container.
pub fn foc_builder_bash_command(command: &str) -> Result<Output, Box<dyn Error>> {
    exec_in_container("foc-builder", "bash", &["-c", command])
}

/// Execute a forge build command in the foc-builder container.
pub fn forge_build_in_container(working_dir: &str) -> Result<Output, Box<dyn Error>> {
    let command = format!("cd {} && forge build", working_dir);
    foc_builder_bash_command(&command)
}

/// Execute a forge script command in the foc-builder container.
pub fn forge_script_deploy(
    script_path: &str,
    rpc_url: &str,
    private_key: &str,
    extra_args: &[&str],
) -> Result<Output, Box<dyn Error>> {
    let mut args = vec![
        "script", script_path,
        "--rpc-url", rpc_url,
        "--private-key", private_key,
        "--broadcast",
    ];
    args.extend_from_slice(extra_args);
    forge_command(&args)
}