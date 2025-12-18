//! Container naming utilities for run-isolated clusters.
//!
//! This module provides functions to generate container names with run ID prefixes.
//! Format: foc-<RUN_ID>-<service> (e.g., foc-251203-1246-thirsty-wolf-lotus)

/// Generate the Lotus container name for a run ID
pub fn lotus_container_name(run_id: &str) -> String {
    format!("foc-{}-lotus", run_id)
}

/// Generate the Lotus Miner container name for a run ID
pub fn lotus_miner_container_name(run_id: &str) -> String {
    format!("foc-{}-lotus-miner", run_id)
}

/// Generate the Builder container name for a run ID
pub fn builder_container_name(run_id: &str) -> String {
    format!("foc-{}-builder", run_id)
}

/// Generate the YugabyteDB container name for a run ID
pub fn yugabyte_container_name(run_id: &str, sp_idx: usize) -> String {
    format!("foc-{}-yugabyte-{}", run_id, sp_idx)
}

/// Generate the Curio container name for a run ID
pub fn curio_container_name(run_id: &str, sp_idx: usize) -> String {
    format!("foc-{}-curio-{}", run_id, sp_idx)
}

/// Generate the Portainer container name for a run ID
pub fn portainer_container_name(run_id: &str) -> String {
    format!("portainer-{}", run_id)
}
