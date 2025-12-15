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
pub fn yugabyte_container_name(run_id: &str) -> String {
    format!("foc-{}-yugabyte", run_id)
}

/// Generate the Curio container name for a run ID
pub fn curio_container_name(run_id: &str) -> String {
    format!("foc-{}-curio", run_id)
}

/// Generate the Portainer container name for a run ID
pub fn portainer_container_name(run_id: &str) -> String {
    format!("portainer-{}", run_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_names() {
        let run_id = "251203-1246-thirsty-wolf";

        assert_eq!(
            lotus_container_name(run_id),
            "foc-251203-1246-thirsty-wolf-lotus"
        );
        assert_eq!(
            lotus_miner_container_name(run_id),
            "foc-251203-1246-thirsty-wolf-lotus-miner"
        );
        assert_eq!(
            builder_container_name(run_id),
            "foc-251203-1246-thirsty-wolf-builder"
        );
        assert_eq!(
            yugabyte_container_name(run_id),
            "foc-251203-1246-thirsty-wolf-yugabyte"
        );
        assert_eq!(
            curio_container_name(run_id),
            "foc-251203-1246-thirsty-wolf-curio"
        );
        assert_eq!(
            portainer_container_name(run_id),
            "foc-251203-1246-thirsty-wolf-portainer"
        );
    }
}
