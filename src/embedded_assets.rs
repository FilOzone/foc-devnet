//! Embedded assets for foc-localnet.
//!
//! This module contains all external files embedded into the binary
//! using include_bytes! to make the binary self-contained.

// Dockerfiles
pub static DOCKERFILE_BUILDER: &[u8] = include_bytes!("../docker/Dockerfile.builder");
pub static DOCKERFILE_CURIO: &[u8] = include_bytes!("../docker/Dockerfile.curio");
pub static DOCKERFILE_LOTUS: &[u8] = include_bytes!("../docker/Dockerfile.lotus");
pub static DOCKERFILE_LOTUS_MINER: &[u8] = include_bytes!("../docker/Dockerfile.lotus-miner");
pub static DOCKERFILE_YUGABYTE: &[u8] = include_bytes!("../docker/Dockerfile.yugabyte");

// Volumes maps
pub static BUILDER_VOLUMES_MAP: &[u8] = include_bytes!("../docker/builder.volumes_map.toml");
pub static CURIO_VOLUMES_MAP: &[u8] = include_bytes!("../docker/curio.volumes_map.toml");
pub static LOTUS_MINER_VOLUMES_MAP: &[u8] =
    include_bytes!("../docker/lotus-miner.volumes_map.toml");
pub static LOTUS_VOLUMES_MAP: &[u8] = include_bytes!("../docker/lotus.volumes_map.toml");
pub static YUGABYTE_VOLUMES_MAP: &[u8] = include_bytes!("../docker/yugabyte.volumes_map.toml");

// Contracts
pub static MOCK_USDFC_CONTRACT: &[u8] = include_bytes!("../contracts/MockUSDFC.sol");

/// Get a Dockerfile by name
pub fn get_dockerfile(name: &str) -> Option<&'static [u8]> {
    match name {
        "builder" => Some(DOCKERFILE_BUILDER),
        "curio" => Some(DOCKERFILE_CURIO),
        "lotus" => Some(DOCKERFILE_LOTUS),
        "lotus-miner" => Some(DOCKERFILE_LOTUS_MINER),
        "yugabyte" => Some(DOCKERFILE_YUGABYTE),
        _ => None,
    }
}

/// Get a volumes map by name
pub fn get_volumes_map(name: &str) -> Option<&'static [u8]> {
    match name {
        "builder" => Some(BUILDER_VOLUMES_MAP),
        "curio" => Some(CURIO_VOLUMES_MAP),
        "lotus-miner" => Some(LOTUS_MINER_VOLUMES_MAP),
        "lotus" => Some(LOTUS_VOLUMES_MAP),
        "yugabyte" => Some(YUGABYTE_VOLUMES_MAP),
        _ => None,
    }
}
