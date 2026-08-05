//! Docker utilities and abstractions for foc-devnet.
//!
//! This module consolidates all Docker-related functionality into a single,
//! well-organized structure. It replaces the old scattered docker.rs and shell.rs
//! modules with a clean modular design.

pub mod build;
pub mod command_logger;
pub mod containers;
pub mod core;
pub mod init;
pub mod logs;
pub mod mounts;
pub mod network;
pub mod portainer;
pub mod shell;
pub mod status;

// Re-export commonly used functions for convenience
pub use core::{
    chown_command, container_exists, container_is_running, copy_from_container, create_container,
    docker_command, exec_in_container, get_current_gid, get_current_uid, image_exists,
    is_port_available, remove_container, run_command, run_container, stop_and_remove_container,
    stop_container, wait_for_port,
};

pub use build::{build_docker_image, build_image_from_embedded, build_image_with_args};
pub use command_logger::{
    format_command, format_command_strings, log_command, log_command_strings, run_and_log_command,
    run_and_log_command_strings,
};
pub use containers::{
    builder_container_name, curio_container_name, db_container_names, lotus_container_name,
    lotus_miner_container_name, portainer_container_name, postgres_container_name,
    scylla_container_name,
};
pub use init::{create_volume_directories_for_images, set_volume_ownership};
pub use logs::{
    list_foc_devnet_containers, persist_foc_container_logs, remove_dead_foc_containers,
    write_post_start_status_log,
};
pub use mounts::{bind_mount, prepare_bind_source, push_bind_mount, push_read_only_bind_mount};
pub use network::{
    connect_container_to_network, create_all_networks, delete_all_networks, is_foc_devnet_network,
    lotus_miner_network_name, lotus_network_name, pdp_miner_network_name,
};
pub use portainer::{start_portainer, stop_portainer};
pub use shell::{cast_command, forge_command, lotus_command, lotus_wallet_command};
pub use status::{get_container_uptime, get_running_foc_containers, get_system_start_time};
