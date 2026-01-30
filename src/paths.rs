use std::path::PathBuf;

/// Returns the path to the foc-devnet home directory, e.g., ~/.foc-devnet
pub fn foc_devnet_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".foc-devnet")
}

/// Returns the path to the foc-devnet logs directory, e.g., ~/.foc-devnet/logs
pub fn foc_devnet_logs() -> PathBuf {
    foc_devnet_home().join("logs")
}

/// Returns the path to the foc-devnet tmp directory, e.g., ~/.foc-devnet/tmp
pub fn foc_devnet_tmp() -> PathBuf {
    foc_devnet_home().join("tmp")
}

/// Returns the path to the foc-devnet runs directory, e.g., ~/.foc-devnet/run
pub fn foc_devnet_runs() -> PathBuf {
    foc_devnet_home().join("run")
}

/// Returns the path to a specific run directory
/// e.g., ~/.foc-devnet/run/20231218_123456
pub fn foc_devnet_run_dir(run_id: &str) -> PathBuf {
    foc_devnet_runs().join(run_id)
}

/// Returns the path to the execution log for a specific run
pub fn foc_devnet_run_log_file(run_id: &str) -> PathBuf {
    foc_devnet_run_dir(run_id).join("setup.log")
}

/// Returns the path to the version file for a specific run
pub fn foc_devnet_run_version_file(run_id: &str) -> PathBuf {
    foc_devnet_run_dir(run_id).join("version.txt")
}

/// Returns the path to the contract addresses file for a specific run
pub fn contract_addresses_file(run_id: &str) -> PathBuf {
    foc_devnet_run_dir(run_id).join("contract_addresses.json")
}

/// Returns the path to the foc-devnet bin directory, e.g., ~/.foc-devnet/bin
pub fn foc_devnet_bin() -> PathBuf {
    foc_devnet_home().join("bin")
}

/// Returns the path to the foc-devnet state directory, e.g., ~/.foc-devnet/state
pub fn foc_devnet_state() -> PathBuf {
    foc_devnet_home().join("state")
}

/// Returns the path to the latest run symlink, e.g., ~/.foc-devnet/state/latest
pub fn foc_devnet_state_latest() -> PathBuf {
    foc_devnet_state().join("latest")
}

/// Returns the path to the foc-devnet keys directory, e.g., ~/.foc-devnet/keys
pub fn foc_devnet_keys() -> PathBuf {
    foc_devnet_home().join("keys")
}

/// Returns the path to the poison file, e.g., ~/.foc-devnet/state/.poison
pub fn poison_file() -> PathBuf {
    foc_devnet_state().join(".poison")
}

/// Returns the path to the FOC metadata file for a specific run
pub fn foc_metadata_file(run_id: &str) -> PathBuf {
    foc_devnet_run_dir(run_id).join("foc_metadata.json")
}

/// Returns the path to the step context file for a specific run
pub fn step_context_file(run_id: &str) -> PathBuf {
    foc_devnet_run_dir(run_id).join("step_context.json")
}

/// Returns the path to the devnet info file for a specific run
/// This is the versioned, stable schema JSON file for external consumers.
pub fn devnet_info_file(run_id: &str) -> PathBuf {
    foc_devnet_run_dir(run_id).join("devnet-info.json")
}

/// Returns the path to the PDP_SP_X provider ID file for a specific run
pub fn pdp_sp_provider_id_file(run_id: &str, sp_idx: usize) -> PathBuf {
    foc_devnet_run_dir(run_id)
        .join("pdp_sps")
        .join(format!("{}.provider_id.json", sp_idx))
}

/// Returns the path to the foc-devnet remote pulls directory, e.g., ~/.foc-devnet/remote-pulls
pub fn foc_devnet_code() -> PathBuf {
    foc_devnet_home().join("code")
}

/// Returns the path to the "lotus" repository
pub fn foc_devnet_lotus_repo() -> PathBuf {
    foc_devnet_code().join("lotus")
}

/// Returns the path to the "curio" repository
pub fn foc_devnet_curio_repo() -> PathBuf {
    foc_devnet_code().join("curio")
}

/// Returns the path to the "filecoin-services" repository
pub fn foc_devnet_filecoin_services_repo() -> PathBuf {
    foc_devnet_code().join("filecoin-services")
}

/// Returns the path to the "multicall3" repository
pub fn foc_devnet_multicall3_repo() -> PathBuf {
    foc_devnet_code().join("multicall3")
}

/// Returns the path to the "synapse-sdk" repository
pub fn foc_devnet_synapse_sdk_repo() -> PathBuf {
    foc_devnet_code().join("synapse-sdk")
}

/// Returns the path to the foc-devnet artifacts directory, e.g., ~/.foc-devnet/artifacts
pub fn foc_devnet_artifacts() -> PathBuf {
    foc_devnet_home().join("artifacts")
}

/// Returns the path where docker volumes are stored, e.g., ~/.foc-devnet/docker/volumes
pub fn foc_devnet_docker_volumes() -> PathBuf {
    foc_devnet_home().join("docker").join("volumes")
}

/// Returns the path to the cache volumes directory, e.g., ~/.foc-devnet/docker/volumes/cache
pub fn foc_devnet_docker_volumes_cache() -> PathBuf {
    foc_devnet_docker_volumes().join("cache")
}

/// Returns the path to the run-specific volumes directory, e.g., ~/.foc-devnet/docker/volumes/run-specific
pub fn foc_devnet_docker_volumes_run_specific_root() -> PathBuf {
    foc_devnet_docker_volumes().join("run-specific")
}

/// Returns the path to a specific run's volumes directory, e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>
pub fn foc_devnet_docker_volumes_run_specific(run_id: &str) -> PathBuf {
    foc_devnet_docker_volumes_run_specific_root().join(run_id)
}

/// Returns the path to the foc-devnet configuration, e.g., ~/.foc-devnet/config.toml
pub fn foc_devnet_config() -> PathBuf {
    foc_devnet_home().join("config.toml")
}

/// Returns the path to the Filecoin proof parameters directory
/// e.g., ~/.foc-devnet/docker/volumes/cache/filecoin-proof-parameters
pub fn foc_devnet_proof_parameters() -> PathBuf {
    foc_devnet_docker_volumes_cache().join("filecoin-proof-parameters")
}

/// Returns the path to store BLS keys for lotus
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/lotus-keys
pub fn foc_devnet_lotus_keys(run_id: &str) -> PathBuf {
    foc_devnet_docker_volumes_run_specific(run_id).join("lotus-keys")
}

/// Returns the path to the pre-sealed sectors for genesis
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/genesis-sectors
pub fn foc_devnet_genesis_sectors(run_id: &str) -> PathBuf {
    foc_devnet_docker_volumes_run_specific(run_id).join("genesis-sectors")
}

/// Returns the path to the pre-sealed sectors for miner 1 (t01000)
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/genesis-sectors/lotus-miner
pub fn foc_devnet_genesis_sectors_lotus_miner(run_id: &str) -> PathBuf {
    foc_devnet_genesis_sectors(run_id).join("lotus-miner")
}

/// Returns the path to the pre-sealed sectors for a PDP SP miner (base-1 indexed)
///
/// PDP SP 1 = t01001, PDP SP 2 = t01002, etc.
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/genesis-sectors/pdp-sp-1
pub fn foc_devnet_genesis_sectors_pdp_sp(run_id: &str, sp_index: usize) -> PathBuf {
    foc_devnet_genesis_sectors(run_id).join(format!("pdp-sp-{}", sp_index))
}

/// **DEPRECATED:** No longer used. Curio miners are now PDP Service Providers.
///
/// This path function remains for backward compatibility during cleanup operations.
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/genesis-sectors/curio-miner
#[allow(dead_code)]
pub fn foc_devnet_genesis_sectors_curio_miner(run_id: &str) -> PathBuf {
    foc_devnet_genesis_sectors(run_id).join("curio-miner")
}

/// Returns the path to the genesis template
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/genesis
pub fn foc_devnet_genesis(run_id: &str) -> PathBuf {
    foc_devnet_docker_volumes_run_specific(run_id).join("genesis")
}

/// Returns the path to the curio volumes directory
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/curio
pub fn foc_devnet_curio_volumes(run_id: &str) -> PathBuf {
    foc_devnet_docker_volumes_run_specific(run_id).join("curio")
}

/// Returns the path to a specific curio SP volume directory (base-1 indexed)
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/curio/1
pub fn foc_devnet_curio_sp_volume(run_id: &str, sp_index: usize) -> PathBuf {
    foc_devnet_curio_volumes(run_id).join(sp_index.to_string())
}

/// Returns the path to the yugabyte volumes directory
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/yugabyte
pub fn foc_devnet_yugabyte_volumes(run_id: &str) -> PathBuf {
    foc_devnet_docker_volumes_run_specific(run_id).join("yugabyte")
}

/// Returns the path to a specific yugabyte instance volume directory (base-1 indexed)
/// e.g., ~/.foc-devnet/docker/volumes/run-specific/<run_id>/yugabyte/1
pub fn foc_devnet_yugabyte_sp_volume(run_id: &str, sp_index: usize) -> PathBuf {
    foc_devnet_yugabyte_volumes(run_id).join(sp_index.to_string())
}

/// Returns the path to the project root directory
/// This is determined by finding the directory containing Cargo.toml
pub fn project_root() -> Result<PathBuf, std::io::Error> {
    // Get the current executable path
    let exe_path = std::env::current_exe()?;

    // Walk up from the executable until we find Cargo.toml
    let mut current = exe_path.parent();
    while let Some(dir) = current {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }

    // Fallback: use CARGO_MANIFEST_DIR if available (during build/test)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        return Ok(PathBuf::from(manifest_dir));
    }

    // Last resort: current directory
    std::env::current_dir()
}

// Constants for container paths
/// Container path where Filecoin proof parameters are mounted
pub const CONTAINER_FILECOIN_PROOF_PARAMS_PATH: &str = "/var/tmp/filecoin-proof-parameters";
