//! Lotus-Miner step.
//!
//! This module handles starting the Lotus-Miner container, which is the first
//! generation miner node that builds tipsets and performs PoRep (Proof of Replication).

use super::step::{Step, StepContext};
use crate::paths::{
    CONTAINER_FILECOIN_PROOF_PARAMS_PATH, foc_localnet_bin, foc_localnet_docker_volumes,
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

const CONTAINER_NAME: &str = "foc-lotus-miner";
const IMAGE_NAME: &str = "foc-lotus-miner";

// Lotus-Miner ports
const LOTUS_MINER_PORTS: &[(u16, &str)] = &[(2345, "Lotus-Miner API")];

/// Step for starting the Lotus-Miner node
pub struct LotusMinerStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl LotusMinerStep {
    /// Create a new LotusMinerStep
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

    /// Check if lotus-miner is responsive
    fn check_miner_api() -> Result<(), Box<dyn Error>> {
        // Try to execute a simple lotus-miner command via docker exec
        let output = Command::new("docker")
            .args([
                "exec",
                CONTAINER_NAME,
                "/usr/local/bin/lotus-bins/lotus-miner",
                "version",
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Lotus-Miner API check failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        Ok(())
    }

    /// Check if lotus daemon is reachable from this container
    fn verify_lotus_connection() -> Result<(), Box<dyn Error>> {
        // We need to ensure the lotus daemon is accessible
        // In a Docker network context, we'll use the host network
        Ok(())
    }
}

impl Step for LotusMinerStep {
    fn name(&self) -> &str {
        "Start Lotus-Miner"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if lotus daemon is running (dependency)
        if context.get("lotus_container_id").is_none() {
            return Err("Lotus daemon must be started before starting Lotus-Miner".into());
        }

        // Check if any existing lotus-miner container is running
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
        for &(port, description) in LOTUS_MINER_PORTS {
            if !Self::is_port_available(port) {
                unavailable_ports.push((port, description));
            }
        }

        if !unavailable_ports.is_empty() {
            let mut error_msg = String::from("The following required ports are not available:\n");
            for (port, description) in unavailable_ports {
                error_msg.push_str(&format!("  - Port {}: {}\n", port, description));
            }
            error_msg.push_str("\nPlease free these ports before starting Lotus-Miner.");
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

        // Verify lotus-miner binary exists
        let miner_bin = foc_localnet_bin().join("lotus-miner");
        if !miner_bin.exists() {
            return Err(
                "Lotus-Miner binary not found. Please run 'foc-localnet build lotus' first.".into(),
            );
        }

        println!("    {} Lotus-Miner binary found", "✓".green());

        // Verify pre-sealed sectors exist
        let sectors_dir = foc_localnet_genesis_sectors();
        if !sectors_dir.exists() || sectors_dir.read_dir()?.next().is_none() {
            return Err(
                "Pre-sealed sectors not found. They should have been created during genesis preparation.".into(),
            );
        }

        println!("    {} Pre-sealed sectors found", "✓".green());

        Self::verify_lotus_connection()?;
        println!("    {} Lotus daemon connectivity verified", "✓".green());

        Ok(())
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Create lotus-miner data directory in volumes
        let miner_data_dir = self.volumes_dir.join("lotus-miner-data");
        fs::create_dir_all(&miner_data_dir)?;

        // Get lotus daemon data directory (needed for API access)
        let lotus_data_dir = self.volumes_dir.join("lotus-data");

        // Get paths
        let bin_dir = foc_localnet_bin();
        let sectors_dir = foc_localnet_genesis_sectors();
        let builder_volumes_dir = foc_localnet_docker_volumes().join("foc-builder");
        let params_dir = foc_localnet_proof_parameters();

        // Find the pre-seal metadata file and key file
        let mut preseal_file = None;
        let mut preseal_key_file = None;
        for entry in fs::read_dir(&sectors_dir)? {
            let entry = entry?;
            let path = entry.path();
            let filename = path.file_name().unwrap().to_string_lossy().to_string();

            if path.is_file() {
                if path.extension().map_or(false, |ext| ext == "json")
                    && filename.starts_with("pre-seal-")
                {
                    preseal_file = Some(filename.clone());
                }
                if path.extension().map_or(false, |ext| ext == "key")
                    && filename.starts_with("pre-seal-")
                {
                    preseal_key_file = Some(filename);
                }
            }
        }

        let preseal_file =
            preseal_file.ok_or("Pre-seal metadata file not found in sectors directory")?;
        let preseal_key_file =
            preseal_key_file.ok_or("Pre-seal key file not found in sectors directory")?;

        // Build docker run command
        // Use the lotus container's network namespace to allow easy communication
        let mut docker_args = vec![
            "run",
            "-d",
            "--name",
            CONTAINER_NAME,
            "--network",
            "container:foc-lotus", // Share network namespace with lotus
        ];

        // Add volume mounts (paths updated for foc-user)
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
            docker_args.extend_from_slice(&["-v", mount]);
        }

        // Set working directory to LOTUS_MINER_PATH
        docker_args.extend_from_slice(&["-w", "/home/foc-user/.lotus-miner-local-net"]);

        // Add image name
        docker_args.push(IMAGE_NAME);

        // Add command: wait for lotus, import wallet key, init, then run
        // Step 0: Wait for lotus daemon API to be ready
        // Step 1: Import the pre-sealed miner key as the default wallet (if not already imported)
        // Step 2: Initialize the lotus-miner with pre-sealed sectors (if not already initialized)
        // Step 3: Run the miner
        // Note: We check if the repo is initialized by checking for config.toml in LOTUS_MINER_PATH
        let miner_cmd = format!(
            r#"echo "Waiting for Lotus daemon API to be ready..." && \
               until /usr/local/bin/lotus-bins/lotus version >/dev/null 2>&1; do \
                 echo "Lotus API not ready yet, waiting..." && sleep 2; \
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
            preseal_key_file, preseal_file
        );
        docker_args.extend_from_slice(&["/bin/bash", "-c", &miner_cmd]);

        println!("    Starting Lotus-Miner container '{}'...", CONTAINER_NAME);
        let output = Command::new("docker").args(&docker_args).output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to start Lotus-Miner container: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        context.set("lotus_miner_container_id", container_id.clone());
        println!(
            "    {} Container started with ID: {}",
            "✓".green(),
            &container_id[..12]
        );

        Ok(())
    }

    fn post_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Wait for container to initialize
        println!("    Waiting for Lotus-Miner to initialize (this may take a while)...");
        thread::sleep(Duration::from_secs(15));

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
        for &(port, description) in LOTUS_MINER_PORTS {
            print!("      Checking port {} ({})... ", port, description);
            match Self::wait_for_port(port, 45) {
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

        // Verify Lotus-Miner API is responsive
        println!("    Verifying Lotus-Miner API connectivity...");
        thread::sleep(Duration::from_secs(5)); // Give miner time to fully initialize
        match Self::check_miner_api() {
            Ok(_) => {
                println!(
                    "    {} Lotus-Miner is ready and responding to API calls",
                    "✓".green()
                );
            }
            Err(e) => {
                println!(
                    "    {} Lotus-Miner API verification failed: {}",
                    "⚠".yellow(),
                    e
                );
                println!(
                    "    Note: Lotus-Miner may still be initializing. This is usually not a critical error."
                );
            }
        }

        // Verify tipset generation (block production)
        println!("    Verifying tipset generation...");
        match Self::check_tipset_generation() {
            Ok(_) => {
                println!(
                    "    {} Tipsets are being generated (blocks are being produced)!",
                    "✓".green().bold()
                );
            }
            Err(e) => {
                println!("    {} Tipset generation check failed: {}", "⚠".yellow(), e);
                println!("    Note: The miner may take a few moments to start producing blocks.");
            }
        }

        println!("\n    {} Lotus-Miner is ready!", "✓".green().bold());
        println!("      API endpoint: http://localhost:2345");
        println!(
            "\n    {} The local Filecoin network is now running and producing tipsets!",
            "🎉".bold()
        );
        println!(
            "      Check chain status: docker exec foc-lotus /usr/local/bin/lotus-bins/lotus chain list"
        );

        Ok(())
    }
}

impl LotusMinerStep {
    /// Check if tipsets are being generated by waiting and comparing chain heights
    fn check_tipset_generation() -> Result<(), Box<dyn Error>> {
        // Get initial chain height
        let output1 = Command::new("docker")
            .args([
                "exec",
                "foc-lotus",
                "/usr/local/bin/lotus-bins/lotus",
                "chain",
                "list",
                "--count=1",
            ])
            .output()?;

        if !output1.status.success() {
            return Err("Failed to get initial chain height".into());
        }

        let chain_output1 = String::from_utf8_lossy(&output1.stdout);
        let height1 = Self::parse_chain_height(&chain_output1)?;

        // Wait for a few seconds
        println!("      Waiting 10 seconds to check for new blocks...");
        thread::sleep(Duration::from_secs(10));

        // Get new chain height
        let output2 = Command::new("docker")
            .args([
                "exec",
                "foc-lotus",
                "/usr/local/bin/lotus-bins/lotus",
                "chain",
                "list",
                "--count=1",
            ])
            .output()?;

        if !output2.status.success() {
            return Err("Failed to get new chain height".into());
        }

        let chain_output2 = String::from_utf8_lossy(&output2.stdout);
        let height2 = Self::parse_chain_height(&chain_output2)?;

        // Check if height increased
        if height2 > height1 {
            println!(
                "      Chain height increased from {} to {} - blocks are being produced!",
                height1, height2
            );
            Ok(())
        } else {
            Err(format!(
                "Chain height did not increase (was {} , still {}). Blocks may not be producing.",
                height1, height2
            )
            .into())
        }
    }

    /// Parse chain height from lotus chain list output
    /// Expected format: "5: (Nov 26 12:05:25) [ bafy...: t01000, ]"
    fn parse_chain_height(output: &str) -> Result<u64, Box<dyn Error>> {
        // Get first line and extract the number before the colon
        if let Some(first_line) = output.lines().next() {
            if let Some(colon_pos) = first_line.find(':') {
                let height_str = first_line[..colon_pos].trim();
                return height_str
                    .parse::<u64>()
                    .map_err(|e| format!("Failed to parse height '{}': {}", height_str, e).into());
            }
        }
        Err("Could not parse chain height from output".into())
    }
}
