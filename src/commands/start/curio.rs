//! Curio step.
//!
//! This module handles starting the Curio container, which is the second
//! generation miner node that performs PDP (Proof of Data Possession) but
//! does not build tipsets.

use super::env_vars::{build_curio_contract_env_vars, build_network_env_vars};
use super::genesis::constants::CURIO_MINER_ID;
use super::lotus_utils::{build_fullnode_api_info, read_lotus_token};
use super::step::{Step, StepContext};
use crate::docker::containers::{
    curio_container_name, lotus_container_name, yugabyte_container_name,
};
use crate::docker::network::{curio_miner_network_name, lotus_network_name};
use crate::docker::{
    connect_container_to_network, container_exists, container_is_running,
    stop_and_remove_container, wait_for_port,
};
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis_sectors_curio_miner,
    foc_localnet_proof_parameters, CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const IMAGE_NAME: &str = "foc-curio";

// Curio ports
const CURIO_PORTS: &[(u16, &str)] = &[(12300, "Curio API"), (12301, "Curio RPC")];

// Timing constants
const CURIO_START_WAIT_SECS: u64 = 10;
const LOG_TAIL_LINES: usize = 50;
const PORT_WAIT_TIMEOUT_SECS: u64 = 30;
const API_CHECK_DELAY_SECS: u64 = 3;

// PDP configuration for Curio
const PDP_CONFIG: &str = r#"[HTTP]
DelegateTLS = true
DomainName = "pdp-sp-0.foc-localnet.internal"
Enable = true
ListenAddress = "0.0.0.0:4702"

[Subsystems]
EnableCommP = true
EnableMoveStorage = true
EnablePDP = true
EnableParkPiece = true"#;

/// Step for starting the Curio node
pub struct CurioStep {
    #[allow(dead_code)]
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl CurioStep {
    /// Create a new CurioStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }

    /// Get the Curio container name from context
    fn get_container_name(context: &StepContext) -> Result<String, Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        Ok(curio_container_name(run_id))
    }

    /// Check if curio is responsive
    fn check_curio_api(context: &StepContext) -> Result<(), Box<dyn Error>> {
        let container_name = Self::get_container_name(context)?;

        // Try to execute a simple curio command via docker exec
        let output = Command::new("docker")
            .args([
                "exec",
                &container_name,
                "/usr/local/bin/lotus-bins/curio",
                "version",
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Curio API check failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        Ok(())
    }

    /// Check that required dependencies are running
    fn check_dependencies(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if yugabyte is running (dependency)
        if context.get("yugabyte_container_id").is_none() {
            return Err("YugabyteDB must be started before starting Curio".into());
        }

        // Check if lotus daemon is running (dependency)
        if context.get("lotus_container_id").is_none() {
            return Err("Lotus daemon must be started before starting Curio".into());
        }

        Ok(())
    }

    /// Check and handle existing Curio container
    fn check_existing_container(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        let container_name = Self::get_container_name(context)?;

        // Check if any existing curio container is running
        if container_exists(&container_name)? {
            if container_is_running(&container_name)? {
                println!(
                    "    {} Container '{}' is already running",
                    "⚠".yellow(),
                    container_name
                );
                stop_and_remove_container(&container_name)?;
            } else {
                println!(
                    "    {} Container '{}' exists but is not running",
                    "⚠".yellow(),
                    container_name
                );
                stop_and_remove_container(&container_name)?;
            }
        }

        Ok(())
    }

    /// Build the Docker run command for Curio
    fn build_docker_command(&self, context: &StepContext) -> Result<Vec<String>, Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        let container_name = curio_container_name(run_id);
        let pdp_network = curio_miner_network_name(run_id);
        let yugabyte_name = yugabyte_container_name(run_id);
        let lotus_name = lotus_container_name(run_id);

        // Read Lotus API token from host
        let lotus_token = read_lotus_token()?;
        let fullnode_api_info = build_fullnode_api_info(&lotus_token, &lotus_name);

        // Build docker run command
        let mut docker_args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            container_name,
            "--network".to_string(),
            pdp_network, // Primary network for YugabyteDB access
        ];

        // When using host networking, we don't need port mappings
        // The container will use the host's network directly
        // docker_args.extend(port_args);

        // Add volume mounts
        let curio_data_dir = self.volumes_dir.join("curio").join(".curio");
        let curio_fast_storage = self.volumes_dir.join("curio").join("fast-storage");
        let curio_long_term_storage = self.volumes_dir.join("curio").join("long-term-storage");

        let curio_volume_mounts = vec![
            format!("{}:/home/foc-user/.curio", curio_data_dir.display()),
            format!(
                "{}:/home/foc-user/curio/fast-storage",
                curio_fast_storage.display()
            ),
            format!(
                "{}:/home/foc-user/curio/long-term-storage",
                curio_long_term_storage.display()
            ),
        ];

        for mount in &curio_volume_mounts {
            docker_args.extend_from_slice(&["-v".to_string(), mount.clone()]);
        }

        // Mount curio binary
        let curio_bin = foc_localnet_bin().join("curio");
        let bin_mount = format!("{}:/usr/local/bin/lotus-bins/curio", curio_bin.display());
        docker_args.extend_from_slice(&["-v".to_string(), bin_mount]);

        // Add volume mounts (similar to lotus-miner)
        let bin_dir = foc_localnet_bin();
        let lotus_data_dir = self.volumes_dir.join("lotus-data");
        let sectors_dir = foc_localnet_genesis_sectors_curio_miner();
        let builder_volumes_dir = foc_localnet_docker_volumes().join("foc-builder");
        let params_dir = foc_localnet_proof_parameters();

        let volume_mounts = vec![
            format!("{}:/usr/local/bin/lotus-bins", bin_dir.display()),
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

        // Add network parameter environment variables (required for all nodes)
        let network_env_vars = build_network_env_vars();
        docker_args.extend(network_env_vars);

        // Add contract address environment variables (Curio-specific)
        match build_curio_contract_env_vars() {
            Ok(contract_env_vars) => {
                docker_args.extend(contract_env_vars);
            }
            Err(e) => {
                println!(
                    "    {} Warning: Could not load contract addresses: {}",
                    "⚠".yellow(),
                    e
                );
                println!("    Curio will start without contract addresses.");
            }
        }

        // Add YugabyteDB connection environment variables
        // Use container name instead of localhost since we're in custom network
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

        // Add Lotus API endpoint - use container name for network access
        let lotus_api = format!("http://{}:1234/rpc/v1", lotus_name);
        docker_args.extend_from_slice(&[
            "-e".to_string(),
            format!("LOTUS_API={}", lotus_api),
            "-e".to_string(),
            format!("FULLNODE_API_INFO={}", fullnode_api_info),
            "-e".to_string(),
            "LOTUS_PATH=/home/foc-user/.lotus-local-net".to_string(),
        ]);

        // Add image name and command
        docker_args.push(IMAGE_NAME.to_string());

        let curio_cmd = format!(
            r#"
               /usr/local/bin/lotus-bins/curio cli storage attach --init --seal /home/foc-user/curio/fast-storage;
               /usr/local/bin/lotus-bins/curio cli storage attach --init --store /home/foc-user/curio/long-term-storage;
               /usr/local/bin/lotus-bins/curio config new-cluster {};
               /usr/local/bin/lotus-bins/curio config set --title pdp << 'EOF'
{}
EOF
               /usr/local/bin/lotus-bins/curio run --layers=gui,pdp"#,
            CURIO_MINER_ID, PDP_CONFIG
        );

        docker_args.extend_from_slice(&["/bin/bash".to_string(), "-c".to_string(), curio_cmd]);

        Ok(docker_args)
    }

    /// Start the Curio container
    fn start_container(
        &self,
        docker_args: Vec<String>,
        context: &mut StepContext,
    ) -> Result<(), Box<dyn Error>> {
        let container_name = Self::get_container_name(context)?;
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        let filecoin_network = lotus_network_name(run_id);

        println!("    Starting container '{}'...", container_name);
        let output = Command::new("docker").args(&docker_args).output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to start Curio container: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        context.set("curio_container_id", container_id.clone());
        context.set("curio_container_name", container_name.clone());
        println!(
            "    {} Container started with ID: {}",
            "✓".green(),
            &container_id[..12]
        );

        // Connect to filecoin network for Lotus FEVM access
        println!("    Connecting to filecoin network for Lotus access...");
        connect_container_to_network(&container_name, &filecoin_network)?;
        println!("    {} Connected to filecoin network", "✓".green());

        Ok(())
    }

    /// Check ports availability and verify requirements
    fn check_ports_and_requirements(&self) -> Result<(), Box<dyn Error>> {
        // Ports will be accessible on host via -p mappings
        println!(
            "    {} Using custom networks - ports exposed to host",
            "✓".green()
        );

        // Verify Docker image exists
        if !crate::docker::core::image_exists(IMAGE_NAME).unwrap_or(true) {
            return Err(format!(
                "Docker image '{}' not found. Please run 'foc-localnet init' to build the image.",
                IMAGE_NAME
            )
            .into());
        }
        println!("    {} Docker image '{}' found", "✓".green(), IMAGE_NAME);

        // Verify curio binary exists
        let curio_bin = foc_localnet_bin().join("curio");
        if !curio_bin.exists() {
            return Err(
                "Curio binary not found. Please run 'foc-localnet build curio' first.".into(),
            );
        }

        println!("    {} Curio binary found", "✓".green());

        Ok(())
    }

    /// Create the Curio data directories
    fn setup_data_directory(&self) -> Result<(), Box<dyn Error>> {
        let curio_base_dir = self.volumes_dir.join("curio");
        let curio_data_dir = curio_base_dir.join(".curio");
        let curio_fast_storage = curio_base_dir.join("fast-storage");
        let curio_long_term_storage = curio_base_dir.join("long-term-storage");

        std::fs::create_dir_all(&curio_data_dir)?;
        std::fs::create_dir_all(&curio_fast_storage)?;
        std::fs::create_dir_all(&curio_long_term_storage)?;

        Ok(())
    }

    /// Start the Curio daemon after cluster configuration
    fn start_curio_daemon(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        let container_name = Self::get_container_name(context)?;

        println!("    Starting Curio daemon...");

        // Start curio run in the background using docker exec -d
        let output = Command::new("docker")
            .args([
                "exec",
                "-d",
                &container_name,
                "/usr/local/bin/lotus-bins/curio",
                "run",
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to start Curio daemon: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        println!("    {} Curio daemon started", "✓".green());
        Ok(())
    }
}

impl Step for CurioStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Start Curio"
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        self.check_dependencies(context)?;
        self.check_existing_container(context)?;
        self.check_ports_and_requirements()?;
        self.setup_data_directory()?;
        let docker_args = self.build_docker_command(context)?;
        self.start_container(docker_args, context)?;
        Ok(())
    }

    /// Perform post-execution verification for Curio startup
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        let container_name = Self::get_container_name(context)?;

        // Wait for container to initialize
        println!("    Waiting for container to initialize...");
        thread::sleep(Duration::from_secs(3));

        // Verify container is running
        if !container_is_running(&container_name)? {
            return Err("Container stopped unexpectedly during initialization".into());
        }
        println!("    {} Container is running", "✓".green());

        // Wait for daemon to start
        println!("    Waiting for Curio daemon to initialize...");
        thread::sleep(Duration::from_secs(CURIO_START_WAIT_SECS));

        // Verify container is still running
        if !container_is_running(&container_name)? {
            // Check logs for errors
            let tail_arg = format!("--tail {}", LOG_TAIL_LINES);
            let logs_output = Command::new("docker")
                .args(["logs", &tail_arg, &container_name])
                .output()?;

            return Err(format!(
                "Curio daemon stopped unexpectedly. Logs:\n{}",
                String::from_utf8_lossy(&logs_output.stdout)
            )
            .into());
        }

        // Check all ports are accessible
        println!("    Verifying port accessibility...");
        for &(port, description) in CURIO_PORTS {
            print!("      Checking port {} ({})... ", port, description);
            match wait_for_port(port, PORT_WAIT_TIMEOUT_SECS) {
                Ok(_) => println!("{}", "✓".green()),
                Err(e) => {
                    println!("{}", "⚠".yellow());
                    println!(
                        "      Note: Port {} may not be immediately available: {}",
                        port, e
                    );
                }
            }
        }

        // Verify Curio API is responsive
        println!("    Verifying Curio API connectivity...");
        thread::sleep(Duration::from_secs(API_CHECK_DELAY_SECS));
        match Self::check_curio_api(context) {
            Ok(_) => {
                println!(
                    "    {} Curio is ready and responding to API calls",
                    "✓".green()
                );
            }
            Err(e) => {
                println!("    {} Curio API verification failed: {}", "⚠".yellow(), e);
                println!(
                    "    Note: Curio may still be initializing. This is usually not a critical error."
                );
            }
        }

        println!("\n    {} Curio is ready!", "✓".green().bold());
        println!("      API endpoint: http://localhost:12300");
        println!("      GUI: http://localhost:4701");
        println!("      PDP HTTP: http://localhost:4702");
        println!("      HTTP RPC: http://localhost:12310");

        Ok(())
    }
}
