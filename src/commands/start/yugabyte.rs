use super::step::{Step, StepContext};
use crate::docker::containers::yugabyte_container_name;
use crate::docker::network::pdp_miner_network_name;
use crate::docker::{
    container_exists, container_is_running, is_port_available, stop_and_remove_container,
    wait_for_port,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const IMAGE_NAME: &str = "foc-yugabyte";

// YugabyteDB ports
const YUGABYTE_PORTS: &[(u16, &str)] = &[
    (5433, "YSQL (PostgreSQL API)"),
    (9042, "YCQL (Cassandra API)"),
    (7000, "YB-Master RPC"),
    (9000, "YB-Master Admin UI"),
    (7100, "YB-TServer RPC"),
    (9100, "YB-TServer Admin UI"),
    (15433, "YugabyteDB Web UI"),
];

/// Step for starting YugabyteDB
pub struct YugabyteStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl YugabyteStep {
    /// Create a new YugabyteStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }

    /// Get the YugabyteDB container name from context
    fn get_container_name(context: &StepContext) -> Result<String, Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        Ok(yugabyte_container_name(run_id))
    }

    /// Verify PostgreSQL connectivity on port 5433
    fn verify_postgres_connection(context: &StepContext) -> Result<(), Box<dyn Error>> {
        let container_name = Self::get_container_name(context)?;

        // Try to connect to the database using docker exec
        let output = Command::new("docker")
            .args([
                "exec",
                &container_name,
                "/yugabyte/bin/ysqlsh",
                "-h",
                "127.0.0.1",
                "-p",
                "5433",
                "-c",
                "SELECT version();",
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to query PostgreSQL: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        Ok(())
    }

    /// Create the YugabyteDB data directory
    fn setup_data_directory(&self) -> Result<(), Box<dyn Error>> {
        let yugabyte_data_dir = self.volumes_dir.join("yugabyte-data");
        std::fs::create_dir_all(&yugabyte_data_dir)?;
        Ok(())
    }

    /// Build the Docker run command for YugabyteDB
    fn build_docker_command(&self, context: &StepContext) -> Result<Vec<String>, Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        let container_name = yugabyte_container_name(run_id);
        let network_name = pdp_miner_network_name(run_id);

        // Build docker run command
        let mut docker_args = vec![
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(),
            container_name,
            "--network".to_string(),
            network_name,
        ];

        // Add port mappings
        let port_args: Vec<String> = YUGABYTE_PORTS
            .iter()
            .flat_map(|&(port, _)| vec!["-p".to_string(), format!("{}:{}", port, port)])
            .collect();

        docker_args.extend(port_args);

        // Add volume mount
        let yugabyte_data_dir = self.volumes_dir.join("yugabyte-data");
        let volume_mount = format!("{}:/yugabyte/data", yugabyte_data_dir.display());
        docker_args.extend_from_slice(&["-v".to_string(), volume_mount]);

        // Add image name
        docker_args.push(IMAGE_NAME.to_string());

        Ok(docker_args)
    }

    /// Start the YugabyteDB container
    fn start_container(
        &self,
        docker_args: Vec<String>,
        context: &mut StepContext,
    ) -> Result<(), Box<dyn Error>> {
        let container_name = Self::get_container_name(context)?;

        println!("    Starting container '{}'...", container_name);
        let output = Command::new("docker").args(&docker_args).output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to start YugabyteDB container: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        context.set("yugabyte_container_id", container_id.clone());
        context.set("yugabyte_container_name", container_name);
        println!(
            "    {} Container started with ID: {}",
            "✓".green(),
            &container_id[..12]
        );

        Ok(())
    }
}

impl Step for YugabyteStep {
    fn name(&self) -> &str {
        "Start YugabyteDB"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        let container_name = Self::get_container_name(context)?;

        // Check if any existing yugabyte container is running
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

        // Check if all required ports are available
        let mut unavailable_ports = Vec::new();
        for &(port, description) in YUGABYTE_PORTS {
            if !is_port_available(port) {
                unavailable_ports.push((port, description));
            }
        }

        if !unavailable_ports.is_empty() {
            let mut error_msg = String::from("The following required ports are not available:\n");
            for (port, description) in unavailable_ports {
                error_msg.push_str(&format!("  - Port {}: {}\n", port, description));
            }
            error_msg.push_str("\nPlease free these ports before starting YugabyteDB.");
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

        Ok(())
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        self.setup_data_directory()?;
        let docker_args = self.build_docker_command(context)?;
        self.start_container(docker_args, context)?;
        Ok(())
    }

    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        let container_name = Self::get_container_name(context)?;

        // Wait for container to be healthy
        println!("    Waiting for YugabyteDB to start...");
        thread::sleep(Duration::from_secs(5));

        // Verify container is running
        if !container_is_running(&container_name)? {
            return Err("Container stopped unexpectedly".into());
        }
        println!("    {} Container is running", "✓".green());

        // Check all ports are accessible
        println!("    Verifying port accessibility...");
        for &(port, description) in YUGABYTE_PORTS {
            print!("      Checking port {} ({})... ", port, description);
            match wait_for_port(port, 30) {
                Ok(_) => println!("{}", "✓".green()),
                Err(e) => {
                    println!("{}", "✗".red());
                    return Err(format!("Port {} is not accessible: {}", port, e).into());
                }
            }
        }

        // Verify PostgreSQL connection
        println!("    Verifying PostgreSQL connectivity...");
        thread::sleep(Duration::from_secs(3)); // Give YugabyteDB a moment to fully initialize
        match Self::verify_postgres_connection(context) {
            Ok(_) => {
                println!(
                    "    {} PostgreSQL is ready and accepting queries",
                    "✓".green()
                );
            }
            Err(e) => {
                println!("    {} PostgreSQL verification failed: {}", "⚠".yellow(), e);
                println!(
                    "    Note: YugabyteDB may still be initializing. This is usually not a critical error."
                );
            }
        }

        println!("\n    {} YugabyteDB is ready!", "✓".green().bold());
        println!("      Web UI: http://localhost:15433");
        println!("      PostgreSQL: localhost:5433");

        Ok(())
    }
}
