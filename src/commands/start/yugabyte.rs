use super::step::{Step, StepContext};
use crate::docker::containers::yugabyte_container_name;
use crate::docker::network::curio_miner_network_name;
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
    instance_index: usize,
    total_instances: usize,
    ports: &[u16],
    volumes_dir: &PathBuf,
    run_id: &str,
) -> Result<(), Box<dyn Error>> {
    // Generate container name with instance suffix if multiple instances
    // Format: foc-yugabyte-{run_id}-{instance_index} for multiple instances
    let container_name = if total_instances == 1 {
        yugabyte_container_name(run_id)
    } else {
        format!("{}-{}", yugabyte_container_name(run_id), instance_index)
    };

    let network_name = curio_miner_network_name(run_id);

    // Create data directory for this instance
    let data_dir = if total_instances == 1 {
        volumes_dir.join("yugabyte-data")
    } else {
        volumes_dir.join(format!("yugabyte-{}-data", instance_index))
    };
    std::fs::create_dir_all(&data_dir)?;

    // Stop and remove existing container if it exists
    if container_exists(&container_name)? {
        println!(
            "    {} Removing existing Yugabyte container {} ...",
            "⚠".yellow(),
            if total_instances == 1 {
                "".to_string()
            } else {
                instance_index.to_string()
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

    // Try to connect to the database using docker exec
    let output = Command::new("docker")
        .args([
            "exec",
            container_name,
            "/yugabyte/bin/ysqlsh",
            "-h",
            "localhost",
            "-p",
            YUGABYTE_YSQL_CONTAINER_PORT,
            "-c",
            "SELECT 1;",
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
    fn get_container_name(context: &StepContext) -> Result<String, Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        Ok(yugabyte_container_name(run_id))
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
        // Determine how many Yugabyte instances to spawn based on active Curio SPs
        let num_instances = self.active_sp_count;

        println!(
            "  {} Spawning {} Yugabyte instance(s) in parallel...",
            "⚡".cyan(),
            num_instances
        );

        // Allocate ports for all instances upfront
        let mut all_ports = Vec::new();
        for instance_index in 1..=num_instances {
            let yugabyte_ports = context.port_allocator.allocate_multiple(7)?;
            all_ports.push((instance_index, yugabyte_ports));
        }

        // Store ports in context for each instance
        for (instance_index, ports) in &all_ports {
            let prefix = if num_instances == 1 {
                "yugabyte".to_string()
            } else {
                format!("yugabyte_{}", instance_index)
            };

            context.set(&format!("{}_ysql_port", prefix), ports[0].to_string());
            context.set(&format!("{}_ycql_port", prefix), ports[1].to_string());
            context.set(&format!("{}_master_rpc_port", prefix), ports[2].to_string());
            context.set(&format!("{}_master_ui_port", prefix), ports[3].to_string());
            context.set(
                &format!("{}_tserver_rpc_port", prefix),
                ports[4].to_string(),
            );
            context.set(&format!("{}_tserver_ui_port", prefix), ports[5].to_string());
            context.set(&format!("{}_web_ui_port", prefix), ports[6].to_string());
        }

        // Spawn containers in parallel using threads
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();

        for (instance_index, ports) in all_ports {
            let volumes_dir = self.volumes_dir.clone();
            let run_id = context.run_id().ok_or("Run ID not found")?.to_string();
            let errors_clone = Arc::clone(&errors);

            let handle = thread::spawn(move || {
                match spawn_yugabyte_instance(
                    instance_index,
                    num_instances,
                    &ports,
                    &volumes_dir,
                    &run_id,
                ) {
                    Ok(_) => {
                        println!(
                            "    {} Yugabyte instance {} started successfully",
                            "✓".green(),
                            if num_instances == 1 {
                                "".to_string()
                            } else {
                                instance_index.to_string()
                            }
                        );
                    }
                    Err(e) => {
                        let error_msg = format!("Instance {}: {}", instance_index, e);
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

    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        let num_instances = self.active_sp_count;
        let run_id = context.run_id().ok_or("Run ID not found")?;

        println!("    Waiting for YugabyteDB instance(s) to start...");
        thread::sleep(Duration::from_secs(5));

        // Verify all instances
        for instance_index in 1..=num_instances {
            let container_name = if num_instances == 1 {
                yugabyte_container_name(run_id)
            } else {
                format!("{}-{}", yugabyte_container_name(run_id), instance_index)
            };

            let instance_label = if num_instances == 1 {
                "".to_string()
            } else {
                format!(" {}", instance_index)
            };

            // Verify container is running
            if !container_is_running(&container_name)? {
                return Err(
                    format!("Yugabyte instance{} stopped unexpectedly", instance_label).into(),
                );
            }
            println!(
                "    {} Yugabyte instance{} is running",
                "✓".green(),
                instance_label
            );

            // Check all ports are accessible for this instance
            let prefix = if num_instances == 1 {
                "yugabyte".to_string()
            } else {
                format!("yugabyte_{}", instance_index)
            };

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
                    "      Instance{} - Checking port {} ({})... ",
                    instance_label, port, description
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
                "    Verifying PostgreSQL connectivity for instance{}...",
                instance_label
            );
            thread::sleep(Duration::from_secs(2));

            if let Err(e) = verify_postgres_connection_for_instance(&container_name) {
                return Err(format!(
                    "PostgreSQL verification failed for instance{}: {}",
                    instance_label, e
                )
                .into());
            }

            println!(
                "    {} PostgreSQL is ready for instance{}",
                "✓".green(),
                instance_label
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

        if num_instances == 1 {
            let web_ui_port: u16 = context.get("yugabyte_web_ui_port").unwrap().parse()?;
            let ysql_port: u16 = context.get("yugabyte_ysql_port").unwrap().parse()?;
            println!("      Web UI: http://localhost:{}", web_ui_port);
            println!("      YSQL endpoint: localhost:{}", ysql_port);
        } else {
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
        }

        Ok(())
    }
}
