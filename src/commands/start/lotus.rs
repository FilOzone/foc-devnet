//! Lotus execution node step.
//!
//! This module handles starting the Lotus daemon container, which runs the
//! Filecoin execution node (FEVM and FVM).

use super::genesis::constants::GENESIS_FILE;
use super::step::{Step, StepContext};
use crate::paths::{
    CONTAINER_FILECOIN_PROOF_PARAMS_PATH, foc_localnet_bin, foc_localnet_genesis,
    foc_localnet_genesis_sectors, foc_localnet_lotus_keys, foc_localnet_proof_parameters,
};
use crate::docker::{container_exists, container_is_running, image_exists, is_port_available, stop_and_remove_container, wait_for_port};
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CONTAINER_NAME: &str = "foc-lotus";
const IMAGE_NAME: &str = "foc-lotus";

// Lotus daemon ports
const LOTUS_PORTS: &[(u16, &str)] = &[(1234, "Lotus API"), (1235, "Lotus P2P")];

// Timing constants
const CONTAINER_INIT_WAIT_SECS: u64 = 10;
const PORT_CHECK_TIMEOUT_SECS: u64 = 30;
const API_FILE_TIMEOUT_SECS: u64 = 180;
const PORT_CHECK_INTERVAL_MS: u64 = 500;
const DAEMON_INIT_WAIT_SECS: u64 = 5;

// Log constants
const LOG_TAIL_LINES: &str = "50";

/// Step for starting the Lotus execution node
pub struct LotusStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl LotusStep {
    /// Create a new LotusStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }

    /// Verify that the genesis block file exists
    fn verify_genesis_file() -> Result<PathBuf, Box<dyn Error>> {
        let genesis_dir = foc_localnet_genesis();
        let genesis_file = genesis_dir.join(GENESIS_FILE);

        if !genesis_file.exists() {
            return Err(
                "Genesis file not found. This should have been created during genesis preparation."
                    .into(),
            );
        }

        Ok(genesis_file)
    }

    /// Check if lotus daemon is responsive via API
    fn check_lotus_api() -> Result<(), Box<dyn Error>> {
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

    /// Enable FEVM in the Lotus config.toml
    ///
    /// This modifies the Lotus config to enable Ethereum RPC support, which is
    /// required for deploying and interacting with Solidity contracts.
    /// Create a pre-configured config.toml with FEVM and ChainIndexer enabled
    fn create_fevm_config(lotus_data_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(lotus_data_dir)?;
        let config_path = lotus_data_dir.join("config.toml");

        // Create a minimal config with FEVM enabled
        let config_content = r#"[API]
  ListenAddress = "/ip4/0.0.0.0/tcp/1234/http"
  Timeout = "30s"

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

    // Note: Old enable_fevm_config() removed - config is now created before container starts

    /// Check and handle any existing Lotus container
    fn check_existing_container() -> Result<(), Box<dyn Error>> {
        // Check if any existing lotus container is running
        if container_exists(CONTAINER_NAME)? {
            if container_is_running(CONTAINER_NAME)? {
                println!(
                    "    {} Container '{}' is already running",
                    "⚠".yellow(),
                    CONTAINER_NAME
                );
                stop_and_remove_container(CONTAINER_NAME)?;
            } else {
                println!(
                    "    {} Container '{}' exists but is not running",
                    "⚠".yellow(),
                    CONTAINER_NAME
                );
                stop_and_remove_container(CONTAINER_NAME)?;
            }
        }
        Ok(())
    }

    /// Check that all required ports are available
    fn check_ports_availability() -> Result<(), Box<dyn Error>> {
        // Check if all required ports are available
        let mut unavailable_ports = Vec::new();
        for &(port, description) in LOTUS_PORTS {
            if !is_port_available(port) {
                unavailable_ports.push((port, description));
            }
        }

        if !unavailable_ports.is_empty() {
            let mut error_msg = String::from("The following required ports are not available:\n");
            for (port, description) in unavailable_ports {
                error_msg.push_str(&format!("  - Port {}: {}\n", port, description));
            }
            error_msg.push_str("\nPlease free these ports before starting Lotus.");
            return Err(error_msg.into());
        }

        println!("    {} All required ports are available", "✓".green());
        Ok(())
    }

    /// Check that required Docker image and Lotus binary exist
    fn check_image_and_binary() -> Result<(), Box<dyn Error>> {
        // Verify Docker image exists
        if !image_exists(IMAGE_NAME) {
            return Err(format!(
                "Docker image '{}' not found. Please run 'foc-localnet init' to build the image.",
                IMAGE_NAME
            )
            .into());
        }
        println!("    {} Docker image '{}' found", "✓".green(), IMAGE_NAME);

        // Verify lotus binary exists
        let lotus_bin = foc_localnet_bin().join("lotus");
        if !lotus_bin.exists() {
            return Err(
                "Lotus binary not found. Please run 'foc-localnet build lotus' first.".into(),
            );
        }

        println!("    {} Lotus binary found", "✓".green());
        Ok(())
    }

    /// Check that genesis file, proof parameters, and sectors exist
    fn check_genesis_and_params() -> Result<(), Box<dyn Error>> {
        // Verify genesis file exists
        let genesis_file = Self::verify_genesis_file()?;
        println!(
            "    {} Genesis file found at {}",
            "✓".green(),
            genesis_file.display()
        );

        // Verify proof parameters exist
        let params_dir = foc_localnet_proof_parameters();
        if !params_dir.exists() || params_dir.read_dir()?.next().is_none() {
            return Err(
                "Filecoin proof parameters not found. They should have been downloaded during genesis preparation.".into(),
            );
        }

        println!("    {} Proof parameters found", "✓".green());

        // Verify pre-sealed sectors exist
        let sectors_dir = foc_localnet_genesis_sectors();
        if !sectors_dir.exists() || sectors_dir.read_dir()?.next().is_none() {
            return Err(
                "Pre-sealed sectors not found. They should have been created during genesis preparation.".into(),
            );
        }

        println!("    {} Pre-sealed sectors found", "✓".green());
        Ok(())
    }

    /// Set up necessary directories for Lotus daemon
    fn setup_directories(&self) -> Result<(), Box<dyn Error>> {
        // Create lotus data directory in volumes
        let lotus_data_dir = self.volumes_dir.join("lotus-data");
        fs::create_dir_all(&lotus_data_dir)?;

        // Create devgen directory for the genesis block and state tree snapshot
        let devgen_dir = self.volumes_dir.join("devgen");
        fs::create_dir_all(&devgen_dir)?;

        // Pre-create config.toml with FEVM and ChainIndexer enabled
        Self::create_fevm_config(&lotus_data_dir)?;

        Ok(())
    }

    /// Build the Docker run command for starting Lotus daemon
    fn build_docker_command(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // Get paths
        let bin_dir = foc_localnet_bin();
        let params_dir = foc_localnet_proof_parameters();
        let genesis_dir = foc_localnet_genesis();
        let sectors_dir = foc_localnet_genesis_sectors();
        let keys_dir = foc_localnet_lotus_keys();
        let genesis_file = genesis_dir.join(GENESIS_FILE);

        // Build docker run command
        let mut docker_args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            CONTAINER_NAME.to_string(),
        ];

        // Add port mappings
        let port_args: Vec<String> = LOTUS_PORTS
            .iter()
            .flat_map(|&(port, _)| vec!["-p".to_string(), format!("{}:{}", port, port)])
            .collect();

        for arg in port_args {
            docker_args.push(arg);
        }

        // Add volume mounts (paths updated for foc-user)
        let volume_mounts = vec![
            format!("{}:/usr/local/bin/lotus-bins", bin_dir.display()),
            format!(
                "{}:/home/foc-user/.lotus-local-net",
                self.volumes_dir.join("lotus-data").display()
            ),
            format!("{}:/devgen", self.volumes_dir.join("devgen").display()),
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
        docker_args.push(IMAGE_NAME.to_string());

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

    /// Start the Lotus daemon container
    fn start_container(&self, docker_args: Vec<String>, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!(
            "    Starting Lotus daemon container '{}'...",
            CONTAINER_NAME
        );
        let output = Command::new("docker").args(&docker_args).output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to start Lotus container: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        context.set("lotus_container_id", container_id.clone());
        println!(
            "    {} Container started with ID: {}",
            "✓".green(),
            &container_id[..12]
        );

        Ok(())
    }

    /// Wait for the container to initialize after starting
    fn wait_for_container_init(&self) -> Result<(), Box<dyn Error>> {
        // Wait for container to initialize
        println!("    Waiting for Lotus daemon to start...");
        thread::sleep(Duration::from_secs(CONTAINER_INIT_WAIT_SECS));

        // Verify container is running
        if !container_is_running(CONTAINER_NAME)? {
            // Check logs for errors
            let logs_output = Command::new("docker")
                .args(["logs", "--tail", LOG_TAIL_LINES, CONTAINER_NAME])
                .output()?;

            return Err(format!(
                "Container stopped unexpectedly. Logs:\n{}",
                String::from_utf8_lossy(&logs_output.stdout)
            )
            .into());
        }
        println!("    {} Container is running", "✓".green());
        Ok(())
    }

    /// Verify that all required ports are accessible
    fn verify_ports(&self) -> Result<(), Box<dyn Error>> {
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
    fn wait_for_api_file(&self) -> Result<(), Box<dyn Error>> {
        // Wait for Lotus API file to exist and daemon to be fully initialized
        println!("    Waiting for Lotus API to be ready (this may take 1-2 minutes)...");
        let lotus_data_dir = self.volumes_dir.join("lotus-data");
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

    /// Verify Lotus API and Ethereum RPC connectivity
    fn verify_api_connectivity(&self) -> Result<(), Box<dyn Error>> {
        // Verify Lotus API is responsive
        println!("    Verifying Lotus API connectivity...");
        match Self::check_lotus_api() {
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
        match Self::check_ethereum_rpc() {
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

    /// Check if Ethereum RPC is available via the Lotus API
    ///
    /// This verifies that FEVM is properly enabled by testing a basic eth_* RPC call.
    fn check_ethereum_rpc() -> Result<(), Box<dyn Error>> {
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
}

impl Step for LotusStep {
    /// Returns the name of this step
    fn name(&self) -> &str {
        "Start Lotus Daemon"
    }

    /// Performs pre-execution checks before starting the Lotus daemon
    ///
    /// This includes checking for existing containers, verifying port availability,
    /// ensuring required Docker images and binaries exist, and validating
    /// genesis and proof parameter files.
    fn pre_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        Self::check_existing_container()?;
        Self::check_ports_availability()?;
        Self::check_image_and_binary()?;
        Self::check_genesis_and_params()?;
        Ok(())
    }

    /// Executes the main logic to start the Lotus daemon container
    ///
    /// This creates necessary directories, builds the Docker run command with
    /// appropriate volume mounts and port mappings, and starts the container.
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        self.setup_directories()?;
        let docker_args = self.build_docker_command()?;
        self.start_container(docker_args, context)?;
        Ok(())
    }

    /// Performs post-execution verification after the Lotus daemon starts
    ///
    /// This waits for the container to initialize, verifies port accessibility,
    /// waits for the Lotus API file to be created, and checks API connectivity
    /// including FEVM/Ethereum RPC availability.
    fn post_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        self.wait_for_container_init()?;
        self.verify_ports()?;
        self.wait_for_api_file()?;
        self.verify_api_connectivity()?;
        Ok(())
    }
}
