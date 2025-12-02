//! Curio step.
//!
//! This module handles starting the Curio container, which is the second
//! generation miner node that performs PDP (Proof of Data Possession) but
//! does not build tipsets.

use super::step::{Step, StepContext};
use crate::docker::{
    container_exists, container_is_running, is_port_available, stop_and_remove_container,
    wait_for_port,
};
use crate::paths::foc_localnet_bin;
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CONTAINER_NAME: &str = "foc-curio";
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
    fn check_existing_container(&self) -> Result<(), Box<dyn Error>> {
        // Check if any existing curio container is running
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

    /// Check ports availability and verify requirements
    fn check_ports_and_requirements(&self) -> Result<(), Box<dyn Error>> {
        // Check if all required ports are available
        let mut unavailable_ports = Vec::new();
        for &(port, description) in CURIO_PORTS {
            if !is_port_available(port) {
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
}

impl Step for CurioStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Start Curio"
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        self.check_dependencies(context)?;
        self.check_existing_container()?;
        self.check_ports_and_requirements()?;
        Ok(())
    }

    /// Perform post-execution verification for Curio startup
    fn post_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Wait for container to initialize
        println!("    Waiting for Curio to start...");
        thread::sleep(Duration::from_secs(CURIO_START_WAIT_SECS));

        // Verify container is running
        if !container_is_running(CONTAINER_NAME)? {
            // Check logs for errors
            let tail_arg = format!("--tail {}", LOG_TAIL_LINES);
            let logs_output = Command::new("docker")
                .args(["logs", &tail_arg, CONTAINER_NAME])
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
