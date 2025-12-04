//! Docker network management for run-isolated clusters.
//!
//! This module handles creating and managing Docker user-defined networks
//! for each cluster run. Networks are named with the run ID prefix to allow
//! multiple concurrent runs.

use super::core::docker_command;
use crossterm::style::Stylize;
use std::error::Error;

/// Network names (suffixes)
const FILECOIN_NET_SUFFIX: &str = "filecoin-net";
const POREP_MINER_NET_SUFFIX: &str = "porep-miner-net";
const PDP_MINER_NET_SUFFIX: &str = "pdp-miner-net";

/// Get the Filecoin network name for a run ID
pub fn filecoin_network_name(run_id: &str) -> String {
    format!("{}-{}", run_id, FILECOIN_NET_SUFFIX)
}

/// Get the PoRep miner network name for a run ID
pub fn porep_miner_network_name(run_id: &str) -> String {
    format!("{}-{}", run_id, POREP_MINER_NET_SUFFIX)
}

/// Get the PDP miner network name for a run ID
pub fn pdp_miner_network_name(run_id: &str) -> String {
    format!("{}-{}", run_id, PDP_MINER_NET_SUFFIX)
}

/// Check if a Docker network exists
pub fn network_exists(network_name: &str) -> Result<bool, Box<dyn Error>> {
    let output = docker_command(&[
        "network",
        "ls",
        "--filter",
        &format!("name=^{}$", network_name),
        "--format",
        "{{.Name}}",
    ])?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(network_name))
}

/// Create a Docker user-defined bridge network
///
/// # Arguments
/// * `network_name` - The name of the network to create
///
/// # Returns
/// Ok(()) on success, error on failure
pub fn create_network(network_name: &str) -> Result<(), Box<dyn Error>> {
    println!("  Creating network '{}'...", network_name.cyan());

    if network_exists(network_name)? {
        println!("    {} Network already exists", "ℹ".cyan());
        return Ok(());
    }

    docker_command(&["network", "create", "--driver", "bridge", network_name])?;
    println!("    {} Network created", "✓".green());

    Ok(())
}

/// Delete a Docker network
///
/// # Arguments
/// * `network_name` - The name of the network to delete
///
/// # Returns
/// Ok(()) on success, error on failure
pub fn delete_network(network_name: &str) -> Result<(), Box<dyn Error>> {
    println!("  Removing network '{}'...", network_name.cyan());

    if !network_exists(network_name)? {
        println!("    {} Network does not exist", "ℹ".cyan());
        return Ok(());
    }

    docker_command(&["network", "rm", network_name])?;
    println!("    {} Network removed", "✓".green());

    Ok(())
}

/// Create all networks for a cluster run
///
/// Networks created:
/// - `<RUN_ID>-filecoin-net`: For Lotus daemon containers
/// - `<RUN_ID>-porep-miner-net`: For Lotus miner containers (also connected to filecoin-net)
/// - `<RUN_ID>-pdp-miner-net`: For Curio and YugabyteDB containers (Curio also connects to filecoin-net)
///
/// # Arguments
/// * `run_id` - The run ID for this cluster
///
/// # Returns
/// Ok(()) on success, error on failure
pub fn create_all_networks(run_id: &str) -> Result<(), Box<dyn Error>> {
    println!("{}", "Creating Docker networks...".blue().bold());

    create_network(&filecoin_network_name(run_id))?;
    create_network(&porep_miner_network_name(run_id))?;
    create_network(&pdp_miner_network_name(run_id))?;

    println!("{}", "  All networks created successfully".green());
    Ok(())
}

/// Delete all networks for a cluster run
///
/// # Arguments
/// * `run_id` - The run ID for this cluster
///
/// # Returns
/// Ok(()) on success, error on failure
pub fn delete_all_networks(run_id: &str) -> Result<(), Box<dyn Error>> {
    println!("{}", "Removing Docker networks...".blue().bold());

    // Delete in reverse order of creation
    delete_network(&pdp_miner_network_name(run_id))?;
    delete_network(&porep_miner_network_name(run_id))?;
    delete_network(&filecoin_network_name(run_id))?;

    println!("{}", "  All networks removed successfully".green());
    Ok(())
}

/// Connect a container to a network
///
/// # Arguments
/// * `container_name` - The name of the container to connect
/// * `network_name` - The name of the network to connect to
///
/// # Returns
/// Ok(()) on success, error on failure
pub fn connect_container_to_network(
    container_name: &str,
    network_name: &str,
) -> Result<(), Box<dyn Error>> {
    docker_command(&["network", "connect", network_name, container_name])?;
    Ok(())
}
