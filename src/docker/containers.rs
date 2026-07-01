//! Container naming utilities for run-isolated clusters.
//!
//! This module provides functions to generate container names with run ID prefixes.
//! Format: foc-<RUN_ID>-<service> (e.g. foc-20260401T1530_ZanyPip-lotus)

use regex::Regex;
use std::sync::LazyLock;

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

/// Generate the Postgres (HarmonyDB) container name for a run ID and SP index
pub fn postgres_container_name(run_id: &str, sp_idx: usize) -> String {
    format!("foc-{}-postgres-{}", run_id, sp_idx)
}

/// Generate the Scylla (IndexStore) container name for a run ID and SP index
pub fn scylla_container_name(run_id: &str, sp_idx: usize) -> String {
    format!("foc-{}-scylla-{}", run_id, sp_idx)
}

/// Per-SP database container names (Postgres + Scylla) for a run. These run
/// stock images, so image-based container discovery cannot identify them;
/// enumerate by name instead. Indices above the active SP count simply won't
/// exist and are skipped by callers.
pub fn db_container_names(run_id: &str) -> Vec<String> {
    (1..=crate::constants::MAX_PDP_SP_COUNT)
        .flat_map(|sp| {
            [
                postgres_container_name(run_id, sp),
                scylla_container_name(run_id, sp),
            ]
        })
        .collect()
}

/// Matches the per-SP database container naming scheme produced by
/// `postgres_container_name`/`scylla_container_name`, including the run-ID
/// shape (`YYYYMMDDTHHMM_Word`), so devnet DB containers can be identified
/// without knowing the run ID (they run stock images, invisible to image-based
/// discovery). The strict run-ID segment keeps unrelated compose containers
/// (e.g. `foc-observer-postgres-calibnet-1`) out of destructive sweeps.
static DEVNET_DB_CONTAINER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^foc-\d{8}T\d{4}_[A-Za-z0-9]+-(postgres|scylla)-\d+$")
        .expect("static regex pattern")
});

/// Check whether a container name is a devnet per-SP database container.
pub fn is_devnet_db_container_name(name: &str) -> bool {
    DEVNET_DB_CONTAINER_RE.is_match(name)
}

/// Generate the Curio container name for a run ID
pub fn curio_container_name(run_id: &str, sp_idx: usize) -> String {
    format!("foc-{}-curio-{}", run_id, sp_idx)
}

/// Generate the Portainer container name for a run ID
pub fn portainer_container_name(run_id: &str) -> String {
    format!("foc-{}-portainer", run_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_devnet_db_container_name_matches_generated_names() {
        let run_id = "20260603T1838_HonkyBean";
        for sp in [1, 2, 5] {
            assert!(is_devnet_db_container_name(&postgres_container_name(
                run_id, sp
            )));
            assert!(is_devnet_db_container_name(&scylla_container_name(
                run_id, sp
            )));
        }
    }

    #[test]
    fn test_is_devnet_db_container_name_rejects_unrelated() {
        assert!(!is_devnet_db_container_name(
            "foc-observer-postgres-calibnet-1"
        ));
        assert!(!is_devnet_db_container_name("foc-observer-postgres-1"));
        assert!(!is_devnet_db_container_name("curio-test-pg"));
        assert!(!is_devnet_db_container_name("curio-test-scylla"));
        assert!(!is_devnet_db_container_name(
            "foc-20260603T1838_HonkyBean-curio-1"
        ));
        assert!(!is_devnet_db_container_name("postgres"));
        assert!(!is_devnet_db_container_name(""));
    }
}
