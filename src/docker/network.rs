//! Docker network management for run-isolated clusters.
//!
//! This module handles creating and managing Docker user-defined networks
//! for each cluster run. Networks are named with the foc- prefix and run ID
//! to allow multiple concurrent runs and easy identification.

use crate::constants::MAX_PDP_SP_COUNT;

use super::core::docker_command;
use crossterm::style::Stylize;
use std::error::Error;

/// Network names (suffixes)
const LOTUS_NET_SUFFIX: &str = "lot-net";
const LOTUS_MINER_NET_SUFFIX: &str = "lot-m-net";
const CURIO_MINER_NET_SUFFIX: &str = "cur-m-net";

/// Get the Lotus network name for a run ID
pub fn lotus_network_name(run_id: &str) -> String {
    format!("foc_{}_{}", run_id, LOTUS_NET_SUFFIX)
}

/// Get the Lotus miner network name for a run ID
pub fn lotus_miner_network_name(run_id: &str) -> String {
    format!("foc_{}_{}", run_id, LOTUS_MINER_NET_SUFFIX)
}

/// Get the Curio miner network name for a run ID
pub fn pdp_miner_network_name(run_id: &str, sp_idx: usize) -> String {
    format!("foc_{}_{}-{}", run_id, CURIO_MINER_NET_SUFFIX, sp_idx)
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
/// - `foc-<RUN_ID>-filecoin-net`: For Lotus daemon containers
/// - `foc-<RUN_ID>-porep-miner-net`: For Lotus miner containers (also connected to filecoin-net)
/// - `foc-<RUN_ID>-pdp-miner-net`: For Curio and YugabyteDB containers (Curio also connects to filecoin-net)
///
/// # Arguments
/// * `run_id` - The run ID for this cluster
/// * `active_pdp_sp_count` - The number of active PDP SPs
///
/// # Returns
/// Ok(()) on success, error on failure
pub fn create_all_networks(run_id: &str, active_pdp_sp_count: usize) -> Result<(), Box<dyn Error>> {
    println!("{}", "Creating Docker networks...".blue().bold());

    create_network(&lotus_network_name(run_id))?;
    create_network(&lotus_miner_network_name(run_id))?;
    for sp_idx in 1..=active_pdp_sp_count {
        create_network(&pdp_miner_network_name(run_id, sp_idx))?;
    }

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
    // We check all possible SP indices to ensure cleanup
    for sp_idx in (1..=MAX_PDP_SP_COUNT).rev() {
        let pdp_network = pdp_miner_network_name(run_id, sp_idx);
        if network_exists(&pdp_network)? {
            delete_network(&pdp_network)?;
        }
    }
    delete_network(&lotus_miner_network_name(run_id))?;
    delete_network(&lotus_network_name(run_id))?;

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
