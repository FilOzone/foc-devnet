use std::path::PathBuf;

/// Returns the path to the foc-localnet home directory, e.g., ~/.foc-localnet
pub fn foc_localnet_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".foc-localnet")
}

/// Returns the path to the foc-localnet logs directory, e.g., ~/.foc-localnet/logs
pub fn foc_localnet_logs() -> PathBuf {
    foc_localnet_home().join("logs")
}

/// Returns the path to the foc-localnet bin directory, e.g., ~/.foc-localnet/bin
pub fn foc_localnet_bin() -> PathBuf {
    foc_localnet_home().join("bin")
}

/// Returns the path to the foc-localnet state directory, e.g., ~/.foc-localnet/state
pub fn foc_localnet_state() -> PathBuf {
    foc_localnet_home().join("state")
}

/// Returns the path to the poison file, e.g., ~/.foc-localnet/state/.poison
pub fn poison_file() -> PathBuf {
    foc_localnet_state().join(".poison")
}

/// Returns the path to the contract addresses file, e.g., ~/.foc-localnet/state/contract_addresses.json
pub fn contract_addresses_file() -> PathBuf {
    foc_localnet_state().join("contract_addresses.json")
}

/// Returns the path to the foc-localnet remote pulls directory, e.g., ~/.foc-localnet/remote-pulls
pub fn foc_localnet_code() -> PathBuf {
    foc_localnet_home().join("code")
}

/// Returns the path to the "lotus" repository
pub fn foc_localnet_lotus_repo() -> PathBuf {
    foc_localnet_code().join("lotus")
}

/// Returns the path to the "curio" repository
pub fn foc_localnet_curio_repo() -> PathBuf {
    foc_localnet_code().join("curio")
}

/// Returns the path to the "filecoin-services" repository
pub fn foc_localnet_filecoin_services_repo() -> PathBuf {
    foc_localnet_code().join("filecoin-services")
}

/// Returns the path to the foc-localnet temporary directory, e.g., ~/.foc-localnet/tmp
pub fn foc_localnet_tmp() -> PathBuf {
    foc_localnet_home().join("tmp")
}

/// Returns the path to the foc-localnet artifacts directory, e.g., ~/.foc-localnet/artifacts
pub fn foc_localnet_artifacts() -> PathBuf {
    foc_localnet_home().join("artifacts")
}

/// Returns the path where docker volumes are stored, e.g., ~/.foc-localnet/artifacts/docker/volumes
pub fn foc_localnet_docker_volumes() -> PathBuf {
    foc_localnet_artifacts().join("docker").join("volumes")
}

/// Returns the path to the foc-localnet configuration, e.g., ~/.foc-localnet/config.toml
pub fn foc_localnet_config() -> PathBuf {
    foc_localnet_home().join("config.toml")
}

/// Returns the path to the Filecoin proof parameters directory
/// e.g., ~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters
pub fn foc_localnet_proof_parameters() -> PathBuf {
    foc_localnet_docker_volumes().join("filecoin-proof-parameters")
}

/// Returns the path to store BLS keys for lotus
/// e.g., ~/.foc-localnet/artifacts/docker/volumes/lotus-keys
pub fn foc_localnet_lotus_keys() -> PathBuf {
    foc_localnet_docker_volumes().join("lotus-keys")
}

/// Returns the path to the pre-sealed sectors for genesis
/// e.g., ~/.foc-localnet/artifacts/docker/volumes/genesis-sectors
pub fn foc_localnet_genesis_sectors() -> PathBuf {
    foc_localnet_docker_volumes().join("genesis-sectors")
}

/// Returns the path to the genesis template
/// e.g., ~/.foc-localnet/artifacts/docker/volumes/genesis
pub fn foc_localnet_genesis() -> PathBuf {
    foc_localnet_docker_volumes().join("genesis")
}

/// Returns the path to store generated keys
/// e.g., ~/.foc-localnet/keys
pub fn foc_localnet_keys() -> PathBuf {
    foc_localnet_home().join("keys")
}

// Constants for container paths
/// Container path where Filecoin proof parameters are mounted
pub const CONTAINER_FILECOIN_PROOF_PARAMS_PATH: &str = "/var/tmp/filecoin-proof-parameters";
