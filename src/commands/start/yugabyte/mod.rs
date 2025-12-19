use super::step::{SetupContext, Step};
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::docker::containers::yugabyte_container_name;
use crate::docker::network::pdp_miner_network_name;
use crate::docker::{
    container_exists, container_is_running, stop_and_remove_container, wait_for_port,
};
use crate::paths::foc_localnet_yugabyte_sp_volume;

const IMAGE_NAME: &str = "foc-yugabyte";

/// Spawn a single Yugabyte instance (used for parallel spawning).
///
/// This function is thread-safe and can be called concurrently.
fn spawn_yugabyte_instance(
    sp_idx: usize,
    total_instances: usize,
    ports: &[u16],
    _volumes_dir: &PathBuf,
    run_id: &str,
) -> Result<(), Box<dyn Error>> {
    // Generate container name with instance suffix
    // Format: foc-{run_id}-yugabyte-{instance_index} (always indexed for consistency)
    let container_name = yugabyte_container_name(run_id, sp_idx);
    let network_name = pdp_miner_network_name(run_id, sp_idx);

    // Create data directory for this instance
    let data_dir = foc_localnet_yugabyte_sp_volume(run_id, sp_idx);
    std::fs::create_dir_all(&data_dir)?;

    // Stop and remove existing container if it exists
    if container_exists(&container_name)? {
        warn!(
            "    ⚠ Removing existing Yugabyte container {} ...",
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
    run_dir: PathBuf,
    /// Number of PDP SPs to activate (1-5)
    active_sp_count: usize,
}

impl YugabyteStep {
    /// Create a new YugabyteStep
    pub fn new(volumes_dir: PathBuf, run_dir: PathBuf, active_sp_count: usize) -> Self {
        Self {
            volumes_dir,
            run_dir,
            active_sp_count,
        }
    }

    /// Get the ports for a specific Yugabyte instance from context
    fn get_instance_ports(
        &self,
        context: &SetupContext,
        instance_index: usize,
    ) -> Result<Vec<u16>, Box<dyn Error>> {
        let prefix = format!("yugabyte_{}", instance_index);
        let port_suffixes = [
            "ysql_port",
            "ycql_port",
            "master_rpc_port",
            "master_ui_port",
            "tserver_rpc_port",
            "tserver_ui_port",
            "web_ui_port",
        ];

        let mut ports = Vec::new();
        for suffix in &port_suffixes {
            let key = format!("{}_{}", prefix, suffix);
            let port: u16 = context
                .get(&key)
                .ok_or(format!("Port key {} not found in context", key))?
                .parse()?;
            ports.push(port);
        }
        Ok(ports)
    }
}

impl Step for YugabyteStep {
    fn name(&self) -> &str {
        "Start YugabyteDB"
    }

    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        // Verify Docker image exists
        if !crate::docker::core::image_exists(IMAGE_NAME).unwrap_or(true) {
            return Err(format!(
                "Docker image '{}' not found. Please run 'foc-localnet init' to build the image.",
                IMAGE_NAME
            )
            .into());
        }
        info!("✓ Docker image '{}' found", IMAGE_NAME);

        // Check if ports are available
        let sp_count = context
            .get("active_pdp_sp_count")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1);

        info!(
            "    Checking port availability for {} Yugabyte instance(s)...",
            sp_count
        );

        // Allocate ports for all instances upfront in pre_execute
        for instance_index in 1..=sp_count {
            let yugabyte_ports = context.allocate_multiple_ports(7)?;
            let prefix = format!("yugabyte_{}", instance_index);

            context.set(
                format!("{}_ysql_port", prefix),
                yugabyte_ports[0].to_string(),
            );
            context.set(
                format!("{}_ycql_port", prefix),
                yugabyte_ports[1].to_string(),
            );
            context.set(
                format!("{}_master_rpc_port", prefix),
                yugabyte_ports[2].to_string(),
            );
            context.set(
                format!("{}_master_ui_port", prefix),
                yugabyte_ports[3].to_string(),
            );
            context.set(
                format!("{}_tserver_rpc_port", prefix),
                yugabyte_ports[4].to_string(),
            );
            context.set(
                format!("{}_tserver_ui_port", prefix),
                yugabyte_ports[5].to_string(),
            );
            context.set(
                format!("{}_web_ui_port", prefix),
                yugabyte_ports[6].to_string(),
            );
        }

        let port_descriptions = [
            "YSQL (PostgreSQL API)",
            "YCQL (Cassandra API)",
            "YB-Master RPC",
            "YB-Master Admin UI",
            "YB-TServer RPC",
            "YB-TServer Admin UI",
            "YugabyteDB Web UI",
        ];

        // Check each port for availability
        for i in 0..sp_count {
            let ports = self.get_instance_ports(context, i + 1)?;
            for (port, desc) in ports.iter().zip(port_descriptions.iter()) {
                if !crate::docker::is_port_available(*port) {
                    warn!("⚠ Port {} ({}) is already in use", port, desc);
                    return Err(format!("Port {} is already in use", port).into());
                }
            }
        }

        info!("✓ All required ports are available");
        Ok(())
    }

    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        let sp_count = context
            .get("active_pdp_sp_count")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1);

        info!("Starting {} YugabyteDB instance(s)...", sp_count);

        // Get ports for all instances from context (already allocated in pre_execute)
        let mut all_ports: Vec<(usize, Vec<u16>)> = Vec::new();
        for instance_index in 1..=sp_count {
            let ports = self.get_instance_ports(context, instance_index)?;
            all_ports.push((instance_index, ports));
        }

        // Spawn containers in parallel using threads
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();
        let num_instances = self.active_sp_count;

        for (sp_idx, ports) in all_ports.into_iter() {
            let volumes_dir = self.volumes_dir.clone();
            let run_id = context.run_id().ok_or("Run ID not found")?.to_string();
            let errors_clone = Arc::clone(&errors);

            let handle = thread::spawn(move || {
                match spawn_yugabyte_instance(sp_idx, num_instances, &ports, &volumes_dir, &run_id)
                {
                    Ok(_) => {
                        info!(
                            "    Yugabyte instance {} started successfully",
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
            if let Err(e) = handle.join() {
                error!("Yugabyte spawn thread panicked: {:?}", e);
                return Err("Thread panicked".into());
            }
        }

        info!("✓ All YugabyteDB instances started");

        Ok(())
    }

    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        let num_instances = self.active_sp_count;
        let run_id = context.run_id().ok_or("Run ID not found")?;

        info!("Waiting for YugabyteDB instance(s) to start...");
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
            info!("Yugabyte instance {} is running", container_name);

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

                info!(
                    "      {} - Checking port {} ({})...",
                    container_name, port, description
                );
                if let Err(e) = wait_for_port(port, 30) {
                    return Err(format!("Port {} is not accessible: {}", port, e).into());
                }
            }

            // Verify PostgreSQL connection for this instance
            info!(
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

            info!("✓ PostgreSQL is ready for {}", container_name);
        }

        info!(
            "    ✓ All {} Yugabyte instance(s) verified successfully",
            num_instances
        );

        // Show connection info
        info!("✓ All YugabyteDB instance(s) ready!");

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
            info!(
                "      Instance {} - Web UI: http://localhost:{}, YSQL: localhost:{}",
                instance_index, web_ui_port, ysql_port
            );
        }

        Ok(())
    }
}
