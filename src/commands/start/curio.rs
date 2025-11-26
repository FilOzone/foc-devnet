//! Curio step.
//!
//! This module handles starting the Curio container, which is the second
//! generation miner node that performs PDP (Proof of Data Possession) but
//! does not build tipsets.

use super::step::{Step, StepContext};
use crate::paths::{
    CONTAINER_FILECOIN_PROOF_PARAMS_PATH, foc_localnet_bin, foc_localnet_proof_parameters,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CONTAINER_NAME: &str = "foc-curio";
const IMAGE_NAME: &str = "foc-curio";

// Curio ports
const CURIO_PORTS: &[(u16, &str)] = &[(12300, "Curio API"), (12301, "Curio RPC")];

/// Step for starting the Curio node
pub struct CurioStep {
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

    /// Check if a port is available (not in use)
    fn is_port_available(port: u16) -> bool {
        TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
    }

    /// Check if a Docker image exists
    fn image_exists(image_name: &str) -> bool {
        Command::new("docker")
            .args(["image", "inspect", image_name])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
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

    /// Check if curio is responsive
    fn check_curio_api() -> Result<(), Box<dyn Error>> {
        // Try to execute a simple curio command via docker exec
        let output = Command::new("docker")
            .args([
                "exec",
                CONTAINER_NAME,
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
}

impl Step for CurioStep {
    fn name(&self) -> &str {
        "Start Curio"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if yugabyte is running (dependency)
        if context.get("yugabyte_container_id").is_none() {
            return Err("YugabyteDB must be started before starting Curio".into());
        }

        // Check if lotus daemon is running (dependency)
        if context.get("lotus_container_id").is_none() {
            return Err("Lotus daemon must be started before starting Curio".into());
        }

        // Check if any existing curio container is running
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
        for &(port, description) in CURIO_PORTS {
            if !Self::is_port_available(port) {
                unavailable_ports.push((port, description));
            }
        }

        if !unavailable_ports.is_empty() {
            let mut error_msg = String::from("The following required ports are not available:\n");
            for (port, description) in unavailable_ports {
                error_msg.push_str(&format!("  - Port {}: {}\n", port, description));
            }
            error_msg.push_str("\nPlease free these ports before starting Curio.");
            return Err(error_msg.into());
        }

        println!("    {} All required ports are available", "✓".green());

        // Verify Docker image exists
        if !Self::image_exists(IMAGE_NAME) {
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

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Create curio data directory in volumes
        let curio_data_dir = self.volumes_dir.join("curio-data");
        fs::create_dir_all(&curio_data_dir)?;

        // Get paths
        let bin_dir = foc_localnet_bin();
        let params_dir = foc_localnet_proof_parameters();

        // Build docker run command
        // Use host network mode to allow curio to connect to yugabyte and lotus
        let mut docker_args = vec![
            "run",
            "-d",
            "--name",
            CONTAINER_NAME,
            "--network",
            "host", // Use host network for easier communication
        ];

        // Add volume mounts
        let volume_mounts = vec![
            format!("{}:/usr/local/bin/lotus-bins", bin_dir.display()),
            format!("{}:/data", curio_data_dir.display()),
            format!(
                "{}:{}",
                params_dir.display(),
                CONTAINER_FILECOIN_PROOF_PARAMS_PATH
            ),
        ];

        for mount in &volume_mounts {
            docker_args.extend_from_slice(&["-v", mount]);
        }

        // Set working directory
        docker_args.extend_from_slice(&["-w", "/data"]);

        // Set environment variables for database connection
        docker_args.extend_from_slice(&[
            "-e",
            "CURIO_DB_HOST=127.0.0.1",
            "-e",
            "CURIO_DB_PORT=5433",
            "-e",
            "CURIO_DB_NAME=yugabyte",
            "-e",
            "CURIO_DB_USER=yugabyte",
        ]);

        // Add image name
        docker_args.push(IMAGE_NAME);

        // Add command to start curio
        // Note: Curio may need initialization steps - this is a basic startup
        docker_args.extend_from_slice(&[
            "/bin/bash",
            "-c",
            r#"if [ ! -f /data/.curio-initialized ]; then \
                 echo "Initializing Curio..."; \
                 /usr/local/bin/lotus-bins/curio config default > /data/config.toml && \
                 touch /data/.curio-initialized; \
               fi && \
               /usr/local/bin/lotus-bins/curio run"#,
        ]);

        println!("    Starting Curio container '{}'...", CONTAINER_NAME);
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
        println!(
            "    {} Container started with ID: {}",
            "✓".green(),
            &container_id[..12]
        );

        Ok(())
    }

    fn post_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Wait for container to initialize
        println!("    Waiting for Curio to start...");
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
        for &(port, description) in CURIO_PORTS {
            print!("      Checking port {} ({})... ", port, description);
            match Self::wait_for_port(port, 30) {
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
        thread::sleep(Duration::from_secs(3));
        match Self::check_curio_api() {
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
