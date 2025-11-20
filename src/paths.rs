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

/// Returns the path to the foc-localnet temporary directory, e.g., ~/.foc-localnet/tmp
pub fn foc_localnet_tmp() -> PathBuf {
    foc_localnet_home().join("tmp")
}

/// Returns the artifacts directory inside foc-localnet, e.g., ~/.foc-localnet/artifacts
pub fn foc_localnet_artifacts() -> PathBuf {
    foc_localnet_home().join("artifacts")
}

/// Returns the path where docker images are stored, e.g., ~/.foc-localnet/artifacts/docker-images
pub fn foc_localnet_docker_images() -> PathBuf {
    foc_localnet_artifacts().join("docker").join("images")
}

/// Returns the path to the foc-localnet configuration, e.g., ~/.foc-localnet/config.toml
pub fn foc_localnet_config() -> PathBuf {
    foc_localnet_home().join("config.toml")
}
