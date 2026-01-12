//! Setup functions for Lotus daemon startup.
//!
//! This module contains functions that prepare the environment
//! for starting the Lotus daemon container.

use super::super::step::SetupContext;
use crate::constants::LOTUS_DOCKER_IMAGE;
use crate::docker::containers::lotus_container_name;
use crate::docker::network::lotus_network_name;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Enable FEVM in the Lotus config.toml
///
/// This modifies the Lotus config to enable Ethereum RPC support, which is
/// required for deploying and interacting with Solidity contracts.
/// Create a pre-configured config.toml with FEVM and ChainIndexer enabled
pub fn create_fevm_config(lotus_data_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(lotus_data_dir)?;
    let config_path = lotus_data_dir.join("config.toml");

    // Create a minimal config with FEVM enabled
    // The API always listens on the container's internal port 1234
    // DisableAuth is set to true for local development (no JWT required)
    let config_content = r#"[API]
  ListenAddress = "/ip4/0.0.0.0/tcp/1234/http"
  Timeout = "30s"
  DisableAuth = true

[Chainstore]
  EnableSplitstore = false

[Fevm]
  EnableEthRPC = true

[ChainIndexer]
  EnableIndexer = true
"#;

    fs::write(&config_path, config_content)?;
    Ok(())
}

/// Set up necessary directories for Lotus daemon
pub fn setup_directories(volumes_dir: &Path) -> Result<(), Box<dyn Error>> {
    // Create lotus data directory in volumes
    let lotus_data_dir = volumes_dir.join("lotus-data");
    fs::create_dir_all(&lotus_data_dir)?;

    // Create devgen directory for the genesis block and state tree snapshot
    let devgen_dir = volumes_dir.join("devgen");
    fs::create_dir_all(&devgen_dir)?;

    // Pre-create config.toml with FEVM and ChainIndexer enabled
    create_fevm_config(&lotus_data_dir)?;

    Ok(())
}

/// Build the Docker run command for starting Lotus daemon
pub fn build_docker_command(
    volumes_dir: &Path,
    context: &SetupContext,
) -> Result<Vec<String>, Box<dyn Error>> {
    use super::super::genesis::constants::GENESIS_FILE;
    use crate::paths::{
        foc_devnet_bin, foc_devnet_genesis, foc_devnet_genesis_sectors, foc_devnet_lotus_keys,
        foc_devnet_proof_parameters, CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
    };

    // Read allocated ports from context
    let lotus_api_port: u16 = context
        .get("lotus_api_port")
        .ok_or("Lotus API port not found in context")?
        .parse()?;
    let lotus_p2p_port: u16 = context
        .get("lotus_p2p_port")
        .ok_or("Lotus P2P port not found in context")?
        .parse()?;

    // Get run-specific container name and network
    let run_id = context.run_id();
    let container_name = lotus_container_name(run_id);
    let network_name = lotus_network_name(run_id);

    // Get paths
    let bin_dir = foc_devnet_bin();
    let params_dir = foc_devnet_proof_parameters();
    let genesis_dir = foc_devnet_genesis(run_id);
    let sectors_dir = foc_devnet_genesis_sectors(run_id);
    let keys_dir = foc_devnet_lotus_keys(run_id);
    let genesis_file = genesis_dir.join(GENESIS_FILE);

    // Build docker run command
    let mut docker_args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name,
        "--network".to_string(),
        network_name,
    ];

    // Add port mappings: map dynamic host ports to fixed container ports
    // Container internal ports: 1234 (API), 1946 (P2P)
    docker_args.extend_from_slice(&[
        "-p".to_string(),
        format!("{}:1234", lotus_api_port), // host:container
        "-p".to_string(),
        format!("{}:1946", lotus_p2p_port), // host:container
    ]);

    // Add volume mounts (paths updated for foc-user)
    let volume_mounts = vec![
        format!("{}:/usr/local/bin/lotus-bins", bin_dir.display()),
        format!(
            "{}:/home/foc-user/.lotus-local-net",
            volumes_dir.join("lotus-data").display()
        ),
        format!("{}:/devgen", volumes_dir.join("devgen").display()),
        format!(
            "{}:{}",
            params_dir.display(),
            CONTAINER_FILECOIN_PROOF_PARAMS_PATH
        ),
        format!("{}:/genesis", genesis_dir.display()),
        format!("{}:/sectors", sectors_dir.display()),
        format!("{}:/keys", keys_dir.display()),
    ];

    for mount in &volume_mounts {
        docker_args.extend_from_slice(&["-v".to_string(), mount.clone()]);
    }

    // Set working directory
    docker_args.extend_from_slice(&["-w".to_string(), "/data".to_string()]);

    // Add image name
    docker_args.push(LOTUS_DOCKER_IMAGE.to_string());

    // Add command to start lotus daemon
    let genesis_filename = genesis_file
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let lotus_cmd = format!(
        r#"/usr/local/bin/lotus-bins/lotus daemon \
            --lotus-make-genesis=/devgen/devgen.car \
            --genesis-template=/genesis/{} \
            --bootstrap=false"#,
        genesis_filename
    );
    docker_args.extend_from_slice(&["/bin/bash".to_string(), "-c".to_string(), lotus_cmd]);

    Ok(docker_args)
}
