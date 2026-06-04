//! Database step: per-SP PostgreSQL (HarmonyDB) + ScyllaDB (IndexStore).
//!
//! Curio needs two endpoints: a Postgres-wire database for HarmonyDB and a
//! Cassandra-wire database for the IndexStore. Each SP gets a stock `postgres`
//! and a tuned `scylladb` container on its per-SP network. Data is ephemeral.
//! The devnet is recreated each run and the Scylla IndexStore is a regenerable
//! cache.

use super::step::{SetupContext, Step};
use crate::constants::{
    DB_NAME, DB_PASSWORD, DB_USER, POSTGRES_CONTAINER_PORT, POSTGRES_DOCKER_IMAGE,
    SCYLLA_CQL_CONTAINER_PORT, SCYLLA_DOCKER_IMAGE,
};
use crate::docker::command_logger::run_and_log_command;
use crate::docker::containers::{postgres_container_name, scylla_container_name};
use crate::docker::core::image_exists;
use crate::docker::network::pdp_miner_network_name;
use crate::docker::{container_exists, container_is_running, stop_and_remove_container};
use std::error::Error;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

/// Readiness polling.
const MAX_RETRIES: u32 = 60;
const RETRY_DELAY_SECS: u64 = 2;

/// Start the PostgreSQL container for one SP. `fsync=off` is safe and faster for
/// throwaway devnet data and speeds up Curio's migration apply.
fn spawn_postgres_instance(
    sp_idx: usize,
    host_port: u16,
    run_id: &str,
    context: &SetupContext,
) -> Result<(), Box<dyn Error>> {
    let container_name = postgres_container_name(run_id, sp_idx);
    let network_name = pdp_miner_network_name(run_id, sp_idx);

    if container_exists(&container_name)? {
        warn!(
            "⚠ Removing existing Postgres container {}...",
            container_name
        );
        stop_and_remove_container(&container_name)?;
    }

    let port_mapping = format!("{}:{}", host_port, POSTGRES_CONTAINER_PORT);
    let user_env = format!("POSTGRES_USER={}", DB_USER);
    let password_env = format!("POSTGRES_PASSWORD={}", DB_PASSWORD);
    let db_env = format!("POSTGRES_DB={}", DB_NAME);

    let docker_args = vec![
        "run",
        "-d",
        "--name",
        &container_name,
        "--network",
        &network_name,
        "-p",
        &port_mapping,
        "-e",
        &user_env,
        "-e",
        &password_env,
        "-e",
        &db_env,
        POSTGRES_DOCKER_IMAGE,
        "-c",
        "fsync=off",
        "-c",
        "full_page_writes=off",
    ];

    let key = format!("postgres_start_sp_{}", sp_idx);
    let output = run_and_log_command("docker", &docker_args, context, &key)?;
    if !output.status.success() {
        return Err(format!(
            "Failed to start Postgres container {}: {}",
            container_name,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}

/// Start the tuned ScyllaDB container for one SP. The flags hold it to a single
/// shard and a small memory reservation. The IndexStore data is KB-scale.
fn spawn_scylla_instance(
    sp_idx: usize,
    host_port: u16,
    run_id: &str,
    context: &SetupContext,
) -> Result<(), Box<dyn Error>> {
    let container_name = scylla_container_name(run_id, sp_idx);
    let network_name = pdp_miner_network_name(run_id, sp_idx);

    if container_exists(&container_name)? {
        warn!("⚠ Removing existing Scylla container {}...", container_name);
        stop_and_remove_container(&container_name)?;
    }

    let port_mapping = format!("{}:{}", host_port, SCYLLA_CQL_CONTAINER_PORT);

    let docker_args = vec![
        "run",
        "-d",
        "--name",
        &container_name,
        "--network",
        &network_name,
        "-p",
        &port_mapping,
        SCYLLA_DOCKER_IMAGE,
        "--smp",
        "1",
        "--memory",
        "512M",
        "--overprovisioned",
        "1",
        "--developer-mode",
        "1",
        "--reserve-memory",
        "0",
    ];

    let key = format!("scylla_start_sp_{}", sp_idx);
    let output = run_and_log_command("docker", &docker_args, context, &key)?;
    if !output.status.success() {
        return Err(format!(
            "Failed to start Scylla container {}: {}",
            container_name,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}

/// Wait until Postgres accepts connections.
fn verify_postgres(container_name: &str, context: &SetupContext) -> Result<(), Box<dyn Error>> {
    for attempt in 1..=MAX_RETRIES {
        let key = format!("postgres_verify_{}_{}", container_name, attempt);
        let output = run_and_log_command(
            "docker",
            &["exec", container_name, "pg_isready", "-U", DB_USER, "-q"],
            context,
            &key,
        )?;
        if output.status.success() {
            return Ok(());
        }
        if attempt < MAX_RETRIES {
            thread::sleep(Duration::from_secs(RETRY_DELAY_SECS));
        }
    }
    Err(format!("Postgres {} did not become ready", container_name).into())
}

/// Wait until Scylla serves CQL. Uses the cqlsh bundled in the Scylla image.
fn verify_scylla(container_name: &str, context: &SetupContext) -> Result<(), Box<dyn Error>> {
    for attempt in 1..=MAX_RETRIES {
        let key = format!("scylla_verify_{}_{}", container_name, attempt);
        let output = run_and_log_command(
            "docker",
            &[
                "exec",
                container_name,
                "cqlsh",
                "-e",
                "SELECT now() FROM system.local",
            ],
            context,
            &key,
        )?;
        if output.status.success() {
            return Ok(());
        }
        if attempt < MAX_RETRIES {
            thread::sleep(Duration::from_secs(RETRY_DELAY_SECS));
        }
    }
    Err(format!("Scylla {} did not become ready", container_name).into())
}

/// Step that starts per-SP Postgres + Scylla containers for Curio.
pub struct DatabaseStep {
    #[allow(dead_code)]
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    run_dir: PathBuf,
    /// Number of PDP SPs to activate (1-5).
    active_sp_count: usize,
}

impl DatabaseStep {
    pub fn new(volumes_dir: PathBuf, run_dir: PathBuf, active_sp_count: usize) -> Self {
        Self {
            volumes_dir,
            run_dir,
            active_sp_count,
        }
    }

    /// Read the allocated host ports for an SP from context.
    fn instance_ports(
        &self,
        context: &SetupContext,
        sp_idx: usize,
    ) -> Result<(u16, u16), Box<dyn Error>> {
        let pg: u16 = context
            .get(&format!("db_{}_postgres_port", sp_idx))
            .ok_or(format!("db_{}_postgres_port not found in context", sp_idx))?
            .parse()?;
        let scylla: u16 = context
            .get(&format!("db_{}_scylla_port", sp_idx))
            .ok_or(format!("db_{}_scylla_port not found in context", sp_idx))?
            .parse()?;
        Ok((pg, scylla))
    }
}

impl Step for DatabaseStep {
    fn name(&self) -> &str {
        "Start Databases (Postgres + Scylla)"
    }

    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        // Stock images are pulled here rather than staged by init, so a fresh
        // machine (or a CI cache carrying only the foc-built images) works.
        for image in [POSTGRES_DOCKER_IMAGE, SCYLLA_DOCKER_IMAGE] {
            if !image_exists(image)? {
                info!("Image '{}' not present, pulling...", image);
                let key = format!("docker_pull_{}", image.replace(['/', ':'], "_"));
                let output = run_and_log_command("docker", &["pull", image], context, &key)?;
                if !output.status.success() {
                    return Err(format!(
                        "Failed to pull image '{}': {}",
                        image,
                        String::from_utf8_lossy(&output.stderr)
                    )
                    .into());
                }
                info!("✓ Pulled {}", image);
            }
        }

        // Allocate and reserve a host port for each engine, for each SP.
        for sp_idx in 1..=self.active_sp_count {
            let ports = context.allocate_multiple_ports(2)?;
            for (port, label) in [(ports[0], "postgres"), (ports[1], "scylla")] {
                if !crate::docker::is_port_available(port) {
                    return Err(format!("Port {} ({}) is already in use", port, label).into());
                }
            }
            context.set(format!("db_{}_postgres_port", sp_idx), ports[0].to_string());
            context.set(format!("db_{}_scylla_port", sp_idx), ports[1].to_string());
        }

        info!(
            "✓ DB images present, ports allocated for {} SP(s)",
            self.active_sp_count
        );
        Ok(())
    }

    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        let run_id = context.run_id();
        info!(
            "Starting Postgres + Scylla for {} SP(s)...",
            self.active_sp_count
        );

        for sp_idx in 1..=self.active_sp_count {
            let (pg_port, scylla_port) = self.instance_ports(context, sp_idx)?;
            spawn_postgres_instance(sp_idx, pg_port, run_id, context)?;
            spawn_scylla_instance(sp_idx, scylla_port, run_id, context)?;
            info!("SP {}: postgres + scylla containers started", sp_idx);
        }

        Ok(())
    }

    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        let run_id = context.run_id();

        for sp_idx in 1..=self.active_sp_count {
            let pg_name = postgres_container_name(run_id, sp_idx);
            let scylla_name = scylla_container_name(run_id, sp_idx);

            for name in [&pg_name, &scylla_name] {
                if !container_is_running(name)? {
                    return Err(format!("Database container {} stopped unexpectedly", name).into());
                }
            }

            info!("SP {}: waiting for Postgres...", sp_idx);
            verify_postgres(&pg_name, context)?;
            info!("SP {}: waiting for Scylla...", sp_idx);
            verify_scylla(&scylla_name, context)?;
            info!("✓ SP {}: databases ready", sp_idx);
        }

        info!("✓ All database instances ready");
        Ok(())
    }

    fn run(&self, context: &SetupContext) -> Result<Duration, Box<dyn Error>> {
        let start = std::time::Instant::now();
        self.pre_execute(context)?;
        self.execute(context)?;
        self.post_execute(context)?;
        info!("✓ {} completed successfully", self.name());
        Ok(start.elapsed())
    }
}
