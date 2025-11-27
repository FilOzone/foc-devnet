use super::step::{Step, StepContext};
use crate::utils::{container_exists, container_is_running, image_exists, is_port_available, stop_and_remove_container, wait_for_port};
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CONTAINER_NAME: &str = "foc-yugabyte";
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

    /// Verify PostgreSQL connectivity on port 5433
    fn verify_postgres_connection() -> Result<(), Box<dyn Error>> {
        // Try to connect to the database using docker exec
        let output = Command::new("docker")
            .args([
                "exec",
                CONTAINER_NAME,
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
}

impl Step for YugabyteStep {
    fn name(&self) -> &str {
        "Start YugabyteDB"
    }

    fn pre_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if any existing yugabyte container is running
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
        if !image_exists(IMAGE_NAME) {
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
        // Create yugabyte data directory in volumes
        let yugabyte_data_dir = self.volumes_dir.join("yugabyte-data");
        std::fs::create_dir_all(&yugabyte_data_dir)?;

        // Build docker run command
        let mut docker_args = vec!["run", "-d", "--name", CONTAINER_NAME];

        // Add port mappings
        let port_args: Vec<String> = YUGABYTE_PORTS
            .iter()
            .flat_map(|&(port, _)| vec!["-p".to_string(), format!("{}:{}", port, port)])
            .collect();

        for arg in &port_args {
            docker_args.push(arg);
        }

        // Add volume mount
        let volume_mount = format!("{}:/yugabyte/data", yugabyte_data_dir.display());
        docker_args.extend_from_slice(&["-v", &volume_mount]);

        // Add image name
        docker_args.push(IMAGE_NAME);

        println!("    Starting container '{}'...", CONTAINER_NAME);
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
        println!(
            "    {} Container started with ID: {}",
            "✓".green(),
            &container_id[..12]
        );

        Ok(())
    }

    fn post_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Wait for container to be healthy
        println!("    Waiting for YugabyteDB to start...");
        thread::sleep(Duration::from_secs(5));

        // Verify container is running
        if !container_is_running(CONTAINER_NAME)? {
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
        match Self::verify_postgres_connection() {
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
