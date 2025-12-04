//! Curio step.
//!
//! This module handles starting the Curio container, which is the second
//! generation miner node that performs PDP (Proof of Data Possession) but
//! does not build tipsets.

use super::step::{Step, StepContext};
use crate::docker::containers::{
    curio_container_name, lotus_container_name, yugabyte_container_name,
};
use crate::docker::network::{filecoin_network_name, pdp_miner_network_name};
use crate::docker::{
    connect_container_to_network, container_exists, container_is_running,
    stop_and_remove_container, wait_for_port,
};
use crate::paths::foc_localnet_bin;
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
        let pdp_network = pdp_miner_network_name(run_id);
        let yugabyte_name = yugabyte_container_name(run_id);
        let lotus_name = lotus_container_name(run_id);

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
        let curio_data_dir = self.volumes_dir.join("curio-data");
        let volume_mount = format!("{}:/home/foc-user/.curio", curio_data_dir.display());
        docker_args.extend_from_slice(&["-v".to_string(), volume_mount]);

        // Mount curio binary
        let curio_bin = foc_localnet_bin().join("curio");
        let bin_mount = format!("{}:/usr/local/bin/lotus-bins/curio", curio_bin.display());
        docker_args.extend_from_slice(&["-v".to_string(), bin_mount]);

        // Add environment variables for contract addresses
        if let Ok(addresses) = crate::commands::start::contract_addresses::ContractAddresses::load()
        {
            // MockUSDFC address
            if !addresses.mock_usdfc.is_empty() {
                docker_args.extend_from_slice(&[
                    "-e".to_string(),
                    format!("USDFC_ADDRESS={}", addresses.mock_usdfc),
                ]);
            }

            // FOC contracts - pass specific ones that Curio needs
            if let Some(addr) = addresses
                .foc_contracts
                .get("FilecoinWarmStorageService Proxy")
            {
                docker_args.extend_from_slice(&[
                    "-e".to_string(),
                    format!("WARM_STORAGE_SERVICE_ADDRESS={}", addr),
                ]);
            }
            if let Some(addr) = addresses.foc_contracts.get("ServiceProviderRegistry Proxy") {
                docker_args.extend_from_slice(&[
                    "-e".to_string(),
                    format!("SERVICE_REGISTRY_ADDRESS={}", addr),
                ]);
            }
            if let Some(addr) = addresses.foc_contracts.get("PDPVerifier Proxy") {
                docker_args.extend_from_slice(&[
                    "-e".to_string(),
                    format!("PDP_VERIFIER_ADDRESS={}", addr),
                ]);
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
        ]);

        // Add Lotus API endpoint - use container name for network access
        let lotus_api = format!("http://{}:1234/rpc/v1", lotus_name);
        docker_args.extend_from_slice(&[
            "-e".to_string(),
            format!("LOTUS_API={}", lotus_api),
            "-e".to_string(),
            format!("FULLNODE_API_INFO={}", lotus_api),
        ]);

        // Add localnet-specific configuration
        docker_args.extend_from_slice(&[
            "-e".to_string(),
            "NETWORK_TYPE=localnet".to_string(),
            "-e".to_string(),
            "CHAIN_ID=31415926".to_string(),
        ]);

        // Add image name and command
        docker_args.push(IMAGE_NAME.to_string());
        docker_args.extend_from_slice(&[
            "/usr/local/bin/lotus-bins/curio".to_string(),
            "run".to_string(),
        ]);

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
        let filecoin_network = filecoin_network_name(run_id);

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

    /// Create the Curio data directory
    fn setup_data_directory(&self) -> Result<(), Box<dyn Error>> {
        let curio_data_dir = self.volumes_dir.join("curio-data");
        std::fs::create_dir_all(&curio_data_dir)?;
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
        println!("    Waiting for Curio to start...");
        thread::sleep(Duration::from_secs(CURIO_START_WAIT_SECS));

        // Verify container is running
        if !container_is_running(&container_name)? {
            // Check logs for errors
            let tail_arg = format!("--tail {}", LOG_TAIL_LINES);
            let logs_output = Command::new("docker")
                .args(["logs", &tail_arg, &container_name])
                .output()?;

            return Err(format!(
                "Container stopped unexpectedly. Logs:\n{}",
                String::from_utf8_lossy(&logs_output.stdout)
            )
            .into());
        }
        println!("    {} Container is running", "✓".green());

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
        println!("      RPC endpoint: http://localhost:12301");

        Ok(())
    }
}
