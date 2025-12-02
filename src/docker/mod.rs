//! Docker utilities and abstractions for foc-localnet.
//!
//! This module consolidates all Docker-related functionality into a single,
//! well-organized structure. It replaces the old scattered docker.rs and shell.rs
//! modules with a clean modular design.

pub mod build;
pub mod core;
pub mod init;
pub mod shell;
pub mod status;

// Re-export commonly used functions for convenience
pub use core::{
    chown_command, container_exists, container_is_running, copy_from_container, create_container,
    docker_command, exec_in_container, get_current_gid, get_current_uid, image_exists,
    is_port_available, remove_container, run_command, run_container, stop_and_remove_container,
    stop_container, wait_for_port,
};

pub use build::{
    build_docker_image, build_image_from_embedded, build_image_with_args, build_yugabyte_image,
};
pub use init::{create_volume_directories_for_images, set_volume_ownership};
pub use shell::{cast_command, forge_command, lotus_command, lotus_wallet_command};
pub use status::{get_container_uptime, get_running_foc_containers, get_system_start_time};
