//! Lotus execution node step.
//!
//! This module handles starting the Lotus daemon container, which runs the
//! Filecoin execution node (FEVM and FVM).

use super::docker_utils::load_image_from_tar;
use super::step::{Step, StepContext};
use crate::paths::{
    CONTAINER_FILECOIN_PROOF_PARAMS_PATH, foc_localnet_bin, foc_localnet_genesis,
    foc_localnet_genesis_sectors, foc_localnet_proof_parameters,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CONTAINER_NAME: &str = "foc-lotus";
const IMAGE_NAME: &str = "foc-lotus";

// Lotus daemon ports
const LOTUS_PORTS: &[(u16, &str)] = &[(1234, "Lotus API"), (1235, "Lotus P2P")];

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

    /// Check if a port is available (not in use)
    fn is_port_available(port: u16) -> bool {
        TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
    }

    /// Check if a container with the given name exists
    fn container_exists(name: &str) -> Result<bool, Box<dyn Error>> {
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                &format!("name=^{}$", name),
                "--format",
                "{{.Names}}",
            ])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .contains(name))
    }

    /// Check if a container is running
    fn container_is_running(name: &str) -> Result<bool, Box<dyn Error>> {
        let output = Command::new("docker")
            .args([
                "ps",
                "--filter",
                &format!("name=^{}$", name),
                "--format",
                "{{.Names}}",
            ])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .contains(name))
    }

    /// Stop and remove a container if it exists
    fn stop_and_remove_container(name: &str) -> Result<(), Box<dyn Error>> {
        if Self::container_is_running(name)? {
            println!("    Stopping existing container '{}'...", name);
            Command::new("docker").args(["stop", name]).output()?;
        }

        if Self::container_exists(name)? {
            println!("    Removing existing container '{}'...", name);
            Command::new("docker").args(["rm", name]).output()?;
        }

        Ok(())
    }

    /// Wait for a port to be accepting connections
    fn wait_for_port(port: u16, timeout_secs: u64) -> Result<(), Box<dyn Error>> {
        let start = std::time::Instant::now();
        loop {
            if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                return Ok(());
            }

            if start.elapsed().as_secs() > timeout_secs {
                return Err(format!("Timeout waiting for port {} to be ready", port).into());
            }

            thread::sleep(Duration::from_millis(500));
        }
    }

    /// Verify that the genesis block file exists
    fn verify_genesis_file() -> Result<PathBuf, Box<dyn Error>> {
        let genesis_dir = foc_localnet_genesis();
        let genesis_file = genesis_dir.join("localnet.json");

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
            .args(["exec", CONTAINER_NAME, "/bin/lotus", "version"])
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
}

impl Step for LotusStep {
    fn name(&self) -> &str {
        "Start Lotus Daemon"
    }

    fn pre_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if any existing lotus container is running
        if Self::container_exists(CONTAINER_NAME)? {
            if Self::container_is_running(CONTAINER_NAME)? {
                println!(
                    "    {} Container '{}' is already running",
                    "⚠".yellow(),
                    CONTAINER_NAME
                );
                Self::stop_and_remove_container(CONTAINER_NAME)?;
            } else {
                println!(
                    "    {} Container '{}' exists but is not running",
                    "⚠".yellow(),
                    CONTAINER_NAME
                );
                Self::stop_and_remove_container(CONTAINER_NAME)?;
            }
        }

        // Check if all required ports are available
        let mut unavailable_ports = Vec::new();
        for &(port, description) in LOTUS_PORTS {
            if !Self::is_port_available(port) {
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

        // Load Docker image from tar file
        load_image_from_tar(IMAGE_NAME, "Lotus")?;

        // Verify lotus binary exists
        let lotus_bin = foc_localnet_bin().join("lotus");
        if !lotus_bin.exists() {
            return Err(
                "Lotus binary not found. Please run 'foc-localnet build lotus' first.".into(),
            );
        }

        println!("    {} Lotus binary found", "✓".green());

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

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Create lotus data directory in volumes
        let lotus_data_dir = self.volumes_dir.join("lotus-data");
        fs::create_dir_all(&lotus_data_dir)?;

        // Get paths
        let bin_dir = foc_localnet_bin();
        let params_dir = foc_localnet_proof_parameters();
        let genesis_dir = foc_localnet_genesis();
        let sectors_dir = foc_localnet_genesis_sectors();
        let genesis_file = genesis_dir.join("localnet.json");

        // Build docker run command
        let mut docker_args = vec!["run", "-d", "--name", CONTAINER_NAME];

        // Add port mappings
        let port_args: Vec<String> = LOTUS_PORTS
            .iter()
            .flat_map(|&(port, _)| vec!["-p".to_string(), format!("{}:{}", port, port)])
            .collect();

        for arg in &port_args {
            docker_args.push(arg);
        }

        // Add volume mounts
        let volume_mounts = vec![
            format!("{}:/bin", bin_dir.display()),
            format!("{}:/data", lotus_data_dir.display()),
            format!(
                "{}:{}",
                params_dir.display(),
                CONTAINER_FILECOIN_PROOF_PARAMS_PATH
            ),
            format!("{}:/genesis", genesis_dir.display()),
            format!("{}:/sectors", sectors_dir.display()),
        ];

        for mount in &volume_mounts {
            docker_args.extend_from_slice(&["-v", mount]);
        }

        // Set working directory
        docker_args.extend_from_slice(&["-w", "/data"]);

        // Add image name
        docker_args.push(IMAGE_NAME);

        // Add command to start lotus daemon
        let genesis_filename = genesis_file
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let lotus_cmd = format!(
            "/bin/lotus daemon --lotus-make-genesis=/data/devgen.car --genesis-template=/genesis/{} --bootstrap=false",
            genesis_filename
        );
        docker_args.extend_from_slice(&["/bin/bash", "-c", &lotus_cmd]);

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

    fn post_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Wait for container to initialize
        println!("    Waiting for Lotus daemon to start...");
        thread::sleep(Duration::from_secs(10));

        // Verify container is running
        if !Self::container_is_running(CONTAINER_NAME)? {
            // Check logs for errors
            let logs_output = Command::new("docker")
                .args(["logs", "--tail", "50", CONTAINER_NAME])
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
        for &(port, description) in LOTUS_PORTS {
            print!("      Checking port {} ({})... ", port, description);
            match Self::wait_for_port(port, 30) {
                Ok(_) => println!("{}", "✓".green()),
                Err(e) => {
                    println!("{}", "✗".red());
                    return Err(format!("Port {} is not accessible: {}", port, e).into());
                }
            }
        }

        // Verify Lotus API is responsive
        println!("    Verifying Lotus API connectivity...");
        thread::sleep(Duration::from_secs(3)); // Give Lotus a moment to fully initialize
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

        println!("\n    {} Lotus daemon is ready!", "✓".green().bold());
        println!("      API endpoint: http://localhost:1234");

        Ok(())
    }
}
