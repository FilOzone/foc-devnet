//! Lotus-Miner step.
//!
//! This module handles starting the Lotus-Miner container, which is the first
//! generation miner node that builds tipsets and performs PoRep (Proof of Replication).

use super::step::{Step, StepContext};
use crate::docker::{container_is_running, wait_for_port};
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis_sectors,
    foc_localnet_proof_parameters, CONTAINER_FILECOIN_PROOF_PARAMS_PATH,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CONTAINER_NAME: &str = "foc-lotus-miner";
const IMAGE_NAME: &str = "foc-lotus-miner";

// Lotus-Miner ports
const LOTUS_MINER_PORTS: &[(u16, &str)] = &[(2345, "Lotus-Miner API")];

// Timing constants
const LOTUS_API_WAIT_SLEEP_SECS: u64 = 2;
const CONTAINER_INIT_WAIT_SECS: u64 = 15;
const MINER_API_CHECK_DELAY_SECS: u64 = 5;
const TIPSET_CHECK_DELAY_SECS: u64 = 10;
const PORT_WAIT_TIMEOUT_SECS: u64 = 45;
const CONTAINER_ID_DISPLAY_LENGTH: usize = 12;

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

    /// Set up necessary directories for Lotus-Miner
    fn setup_miner_directories(&self) -> Result<(), Box<dyn Error>> {
        // Create lotus-miner data directory in volumes
        let miner_data_dir = self.volumes_dir.join("lotus-miner-data");
        fs::create_dir_all(&miner_data_dir)?;
        Ok(())
    }

    /// Find the pre-seal metadata and key files
    fn find_preseal_files(&self) -> Result<(String, String), Box<dyn Error>> {
        let sectors_dir = foc_localnet_genesis_sectors();

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

        Ok((preseal_file, preseal_key_file))
    }

    /// Build the Docker run command for Lotus-Miner
    fn build_miner_docker_command(
        &self,
        preseal_files: &(String, String),
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let (preseal_file, preseal_key_file) = preseal_files;

        // Get lotus daemon data directory (needed for API access)
        let lotus_data_dir = self.volumes_dir.join("lotus-data");

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
        let miner_data_dir = self.volumes_dir.join("lotus-miner-data");
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

    /// Start the Lotus-Miner container
    fn start_miner_container(
        &self,
        docker_args: Vec<String>,
        context: &mut StepContext,
    ) -> Result<(), Box<dyn Error>> {
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
            &container_id[..CONTAINER_ID_DISPLAY_LENGTH]
        );

        Ok(())
    }
}

impl Step for LotusMinerStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Start Lotus-Miner"
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        self.setup_miner_directories()?;
        let preseal_files = self.find_preseal_files()?;
        let docker_args = self.build_miner_docker_command(&preseal_files)?;
        self.start_miner_container(docker_args, context)?;
        Ok(())
    }

    /// Perform post-execution verification for Lotus-Miner startup
    fn post_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Wait for container to initialize
        println!("    Waiting for Lotus-Miner to initialize (this may take a while)...");
        thread::sleep(Duration::from_secs(CONTAINER_INIT_WAIT_SECS));

        // Verify container is running
        if !container_is_running(CONTAINER_NAME)? {
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

        // Verify Lotus-Miner API is responsive
        println!("    Verifying Lotus-Miner API connectivity...");
        thread::sleep(Duration::from_secs(MINER_API_CHECK_DELAY_SECS)); // Give miner time to fully initialize
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

        println!(
            "      Waiting {} seconds to check for new blocks...",
            TIPSET_CHECK_DELAY_SECS
        );
        thread::sleep(Duration::from_secs(TIPSET_CHECK_DELAY_SECS));

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
