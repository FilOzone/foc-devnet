use super::step::{Step, StepContext};
use crate::docker::containers::yugabyte_container_name;
use crate::docker::network::pdp_miner_network_name;
use crate::docker::{
    container_exists, container_is_running, stop_and_remove_container, wait_for_port,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const IMAGE_NAME: &str = "foc-yugabyte";

/// Spawn a single Yugabyte instance (used for parallel spawning).
///
/// This function is thread-safe and can be called concurrently.
fn spawn_yugabyte_instance(
    sp_idx: usize,
    total_instances: usize,
    ports: &[u16],
    volumes_dir: &PathBuf,
    run_id: &str,
) -> Result<(), Box<dyn Error>> {
    // Generate container name with instance suffix
    // Format: foc-{run_id}-yugabyte-{instance_index} (always indexed for consistency)
    let container_name = yugabyte_container_name(run_id, sp_idx);
    let network_name = pdp_miner_network_name(run_id, sp_idx);

    // Create data directory for this instance
    let data_dir = volumes_dir.join(format!("yugabyte-data/{}", sp_idx));
    std::fs::create_dir_all(&data_dir)?;

    // Stop and remove existing container if it exists
    if container_exists(&container_name)? {
        println!(
            "    {} Removing existing Yugabyte container {} ...",
            "⚠".yellow(),
            if total_instances == 1 {
                "".to_string()
            } else {
                sp_idx.to_string()
            }
        );
        stop_and_remove_container(&container_name)?;
    }

    // Build Docker run command
    let mut docker_args = vec![
        "run",
        "-d",
        "--name",
        &container_name,
        "--network",
        &network_name,
    ];

    // Add port mappings
    let port_mappings = vec![
        format!("{}:5433", ports[0]),  // YSQL
        format!("{}:9042", ports[1]),  // YCQL
        format!("{}:7100", ports[2]),  // Master RPC
        format!("{}:7000", ports[3]),  // Master UI
        format!("{}:9100", ports[4]),  // TServer RPC
        format!("{}:9000", ports[5]),  // TServer UI
        format!("{}:15433", ports[6]), // Web UI
    ];

    for mapping in &port_mappings {
        docker_args.push("-p");
        docker_args.push(mapping);
    }

    // Add volume mount
    let data_dir_str = data_dir.to_str().ok_or("Invalid path")?;
    let volume_mount = format!("{}:/home/yugabyte/yb_data", data_dir_str);
    docker_args.extend_from_slice(&["-v", &volume_mount]);

    // Add environment variables
    docker_args.extend_from_slice(&[
        "-e",
        "YSQL_PASSWORD=yugabyte",
        "-e",
        "YSQL_DB=yugabyte",
        "-e",
        "YSQL_USER=yugabyte",
    ]);

    // Add image name
    docker_args.push(IMAGE_NAME);

    // Add YugabyteDB startup command with full configuration
    docker_args.extend_from_slice(&[
        "/yugabyte/bin/yugabyted",
        "start",
        "--ui=true",
        "--callhome=false",
        "--advertise_address=0.0.0.0",
        "--master_flags=rpc_bind_addresses=0.0.0.0",
        "--tserver_flags=rpc_bind_addresses=0.0.0.0,pgsql_proxy_bind_address=0.0.0.0:5433,cql_proxy_bind_address=0.0.0.0:9042",
        "--daemon=false",
    ]);

    // Run the container
    let output = Command::new("docker").args(&docker_args).output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to start Yugabyte container: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}

/// Verify PostgreSQL connectivity for a specific Yugabyte instance.
fn verify_postgres_connection_for_instance(container_name: &str) -> Result<(), Box<dyn Error>> {
    const YUGABYTE_YSQL_CONTAINER_PORT: &str = "5433";
    const MAX_RETRIES: u32 = 30;
    const RETRY_DELAY_SECS: u64 = 2;

    // YugabyteDB YSQL service takes time to initialize after the container starts
    // Retry connection attempts with delays
    for attempt in 1..=MAX_RETRIES {
        let output = Command::new("docker")
            .args([
                "exec",
                "-e",
                "PGPASSWORD=yugabyte",
                container_name,
                "/yugabyte/bin/ysqlsh",
                "-h",
                "localhost",
                "-p",
                YUGABYTE_YSQL_CONTAINER_PORT,
                "-U",
                "yugabyte",
                "-d",
                "yugabyte",
                "-c",
                "SELECT 1;",
            ])
            .output()?;

        if output.status.success() {
            return Ok(());
        }

        // If not the last attempt, wait before retrying
        if attempt < MAX_RETRIES {
            thread::sleep(Duration::from_secs(RETRY_DELAY_SECS));
        } else {
            // Last attempt failed, return error
            return Err(format!(
                "Failed to query PostgreSQL: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
    }

    Ok(())
}

/// Step for starting YugabyteDB
pub struct YugabyteStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
    /// Number of PDP SPs to activate (1-5)
    active_sp_count: usize,
}

impl YugabyteStep {
    /// Create a new YugabyteStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf, active_sp_count: usize) -> Self {
        Self {
            volumes_dir,
            logs_dir,
            active_sp_count,
        }
    }

    /// Get the YugabyteDB container name from context
    fn get_container_name(
        context: &StepContext,
        instance_index: usize,
    ) -> Result<String, Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        Ok(yugabyte_container_name(run_id, instance_index))
    }
}

impl Step for YugabyteStep {
    fn name(&self) -> &str {
        "Start YugabyteDB"
    }

    fn pre_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        for instance_index in 1..=self.active_sp_count {
            let container_name = Self::get_container_name(context, instance_index)?;

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
        }

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

    fn execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        // Determine how many Yugabyte instances to spawn based on active Curio SPs
        let num_instances = self.active_sp_count;

        println!(
            "  {} Spawning {} Yugabyte instance(s) in parallel...",
            "⚡".cyan(),
            num_instances
        );

        // Allocate ports for all instances upfront
        let mut all_ports: Vec<(usize, Vec<u16>)> = Vec::new();
        for instance_index in 1..=num_instances {
            let yugabyte_ports = context.allocate_multiple_ports(7)?;
            all_ports.push((instance_index, yugabyte_ports));
        }

        // Store ports in context for each instance
        for (instance_index, ports) in &all_ports {
            let prefix = format!("yugabyte_{}", instance_index);

            context.set(format!("{}_ysql_port", prefix), ports[0].to_string());
            context.set(format!("{}_ycql_port", prefix), ports[1].to_string());
            context.set(format!("{}_master_rpc_port", prefix), ports[2].to_string());
            context.set(format!("{}_master_ui_port", prefix), ports[3].to_string());
            context.set(format!("{}_tserver_rpc_port", prefix), ports[4].to_string());
            context.set(format!("{}_tserver_ui_port", prefix), ports[5].to_string());
            context.set(format!("{}_web_ui_port", prefix), ports[6].to_string());
        }

        // Spawn containers in parallel using threads
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();

        for (sp_idx, ports) in all_ports.into_iter() {
            let volumes_dir = self.volumes_dir.clone();
            let run_id = context.run_id().ok_or("Run ID not found")?.to_string();
            let errors_clone = Arc::clone(&errors);

            let handle = thread::spawn(move || {
                match spawn_yugabyte_instance(sp_idx, num_instances, &ports, &volumes_dir, &run_id)
                {
                    Ok(_) => {
                        println!(
                            "    {} Yugabyte instance {} started successfully",
                            "✓".green(),
                            if num_instances == 1 {
                                "".to_string()
                            } else {
                                sp_idx.to_string()
                            }
                        );
                    }
                    Err(e) => {
                        let error_msg = format!("Instance {}: {}", sp_idx, e);
                        errors_clone.lock().unwrap().push(error_msg);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().map_err(|_| "Thread panicked")?;
        }

        // Check for errors
        let error_list = errors.lock().unwrap();
        if !error_list.is_empty() {
            return Err(format!(
                "Failed to start Yugabyte instances:\n{}",
                error_list.join("\n")
            )
            .into());
        }

        println!(
            "  {} All {} Yugabyte instance(s) started in parallel",
            "✓".green(),
            num_instances
        );

        Ok(())
    }

    fn post_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        let num_instances = self.active_sp_count;
        let run_id = context.run_id().ok_or("Run ID not found")?;

        println!("    Waiting for YugabyteDB instance(s) to start...");
        thread::sleep(Duration::from_secs(5));

        // Verify all instances
        for instance_index in 1..=num_instances {
            let container_name = yugabyte_container_name(run_id, instance_index);

            // Verify container is running
            if !container_is_running(&container_name)? {
                return Err(
                    format!("Yugabyte instance{} stopped unexpectedly", container_name).into(),
                );
            }
            println!(
                "    {} Yugabyte instance{} is running",
                "✓".green(),
                container_name
            );

            // Check all ports are accessible for this instance
            let prefix = format!("yugabyte_{}", instance_index);

            let port_names = [
                ("ysql_port", "YSQL (PostgreSQL API)"),
                ("ycql_port", "YCQL (Cassandra API)"),
                ("master_rpc_port", "YB-Master RPC"),
                ("master_ui_port", "YB-Master Admin UI"),
                ("tserver_rpc_port", "YB-TServer RPC"),
                ("tserver_ui_port", "YB-TServer Admin UI"),
                ("web_ui_port", "YugabyteDB Web UI"),
            ];

            for (port_suffix, description) in port_names {
                let port_key = format!("{}_{}", prefix, port_suffix);
                let port: u16 = context
                    .get(&port_key)
                    .ok_or(format!("Port key {} not found in context", port_key))?
                    .parse()?;

                print!(
                    "      {} - Checking port {} ({})... ",
                    container_name, port, description
                );
                match wait_for_port(port, 30) {
                    Ok(_) => println!("{}", "✓".green()),
                    Err(e) => {
                        println!("{}", "✗".red());
                        return Err(format!("Port {} is not accessible: {}", port, e).into());
                    }
                }
            }

            // Verify PostgreSQL connection for this instance
            println!(
                "    Verifying PostgreSQL connectivity for {}...",
                container_name
            );
            thread::sleep(Duration::from_secs(2));

            if let Err(e) = verify_postgres_connection_for_instance(&container_name) {
                return Err(format!(
                    "PostgreSQL verification failed for {}: {}",
                    container_name, e
                )
                .into());
            }

            println!(
                "    {} PostgreSQL is ready for {}",
                "✓".green(),
                container_name
            );
        }

        println!(
            "    {} All {} Yugabyte instance(s) verified successfully",
            "✓".green(),
            num_instances
        );

        // Show connection info
        println!(
            "\n    {} All YugabyteDB instance(s) ready!",
            "✓".green().bold()
        );

        for instance_index in 1..=num_instances {
            let prefix = format!("yugabyte_{}", instance_index);
            let web_ui_port: u16 = context
                .get(&format!("{}_web_ui_port", prefix))
                .unwrap()
                .parse()?;
            let ysql_port: u16 = context
                .get(&format!("{}_ysql_port", prefix))
                .unwrap()
                .parse()?;
            println!(
                "      Instance {} - Web UI: http://localhost:{}, YSQL: localhost:{}",
                instance_index, web_ui_port, ysql_port
            );
        }

        Ok(())
    }
}
