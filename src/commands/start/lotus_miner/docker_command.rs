//! Docker command building for Lotus-Miner.
//!
//! This module provides utilities for building Docker run commands for Lotus-Miner.

use std::error::Error;
use std::path::PathBuf;

use super::constants::{CONTAINER_NAME, IMAGE_NAME, LOTUS_API_WAIT_SLEEP_SECS};
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis_sectors,
    foc_localnet_proof_parameters, CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
};

/// Build the Docker run command for Lotus-Miner
pub fn build_miner_docker_command(
    volumes_dir: &PathBuf,
    preseal_files: &(String, String),
) -> Result<Vec<String>, Box<dyn Error>> {
    let (preseal_file, preseal_key_file) = preseal_files;

    // Get lotus daemon data directory (needed for API access)
    let lotus_data_dir = volumes_dir.join("lotus-data");

    // Get paths
    let bin_dir = foc_localnet_bin();
    let sectors_dir = foc_localnet_genesis_sectors();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("foc-builder");
    let params_dir = foc_localnet_proof_parameters();

    // Build docker run command
    // Use the lotus container's network namespace to allow easy communication
    let mut docker_args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        CONTAINER_NAME.to_string(),
        "--network".to_string(),
        "container:foc-lotus".to_string(), // Share network namespace with lotus
    ];

    // Add volume mounts (paths updated for foc-user)
    let miner_data_dir = volumes_dir.join("lotus-miner-data");
    let volume_mounts = vec![
        format!("{}:/usr/local/bin/lotus-bins", bin_dir.display()),
        format!(
            "{}:/home/foc-user/.lotus-miner-local-net",
            miner_data_dir.display()
        ),
        format!(
            "{}:/home/foc-user/.lotus-local-net",
            lotus_data_dir.display()
        ),
        format!("{}:/sectors", sectors_dir.display()),
        format!(
            "{}:{}",
            params_dir.display(),
            CONTAINER_FILECOIN_PROOF_PARAMS_PATH
        ),
        format!("{}:/cargo", builder_volumes_dir.join("cargo").display()),
    ];

    for mount in &volume_mounts {
        docker_args.extend_from_slice(&["-v".to_string(), mount.clone()]);
    }

    // Set working directory to LOTUS_MINER_PATH
    docker_args.extend_from_slice(&[
        "-w".to_string(),
        "/home/foc-user/.lotus-miner-local-net".to_string(),
    ]);

    // Add image name
    docker_args.push(IMAGE_NAME.to_string());

    // Add command: wait for lotus, import wallet key, init, then run
    let miner_cmd = format!(
        r#"echo "Waiting for Lotus daemon API to be ready..." && \
           until /usr/local/bin/lotus-bins/lotus version >/dev/null 2>&1; do \
             echo "Lotus API not ready yet, waiting..." && sleep {}; \
           done && \
           echo "Lotus daemon API is ready!" && \
           if [ ! -f $LOTUS_MINER_PATH/config.toml ]; then \
             echo "Importing pre-sealed miner key..." && \
             (/usr/local/bin/lotus-bins/lotus wallet import --as-default /sectors/{} 2>&1 | grep -v "key already exists" || true) && \
             echo "Initializing lotus-miner..." && \
             /usr/local/bin/lotus-bins/lotus-miner init --genesis-miner --actor=t01000 --sector-size=2KiB \
               --pre-sealed-sectors=/sectors --pre-sealed-metadata=/sectors/{} --nosync; \
           fi && \
           echo "Starting lotus-miner..." && \
           /usr/local/bin/lotus-bins/lotus-miner run --nosync"#,
        LOTUS_API_WAIT_SLEEP_SECS, preseal_key_file, preseal_file
    );
    docker_args.extend_from_slice(&["/bin/bash".to_string(), "-c".to_string(), miner_cmd]);

    Ok(docker_args)
}
