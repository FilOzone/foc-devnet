//! Docker utilities and abstractions for foc-localnet.
//!
//! This module consolidates all Docker-related functionality into a single,
//! well-organized structure. It replaces the old scattered docker.rs and shell.rs
//! modules with a clean modular design.

pub mod core;
pub mod build;
pub mod status;
pub mod init;
pub mod shell;

// Re-export commonly used functions for convenience
pub use core::{
    run_command, docker_command, is_port_available, image_exists,
    container_exists, container_is_running, stop_container, remove_container,
    stop_and_remove_container, exec_in_container, run_container,
    create_container, copy_from_container, wait_for_port,
    get_current_uid, get_current_gid, chown_command
};

pub use build::{build_image_from_embedded, build_image_with_args, build_yugabyte_image, build_docker_image};
pub use status::{get_running_foc_containers, get_container_uptime, get_system_start_time};
pub use init::{create_volume_directories_for_images, set_volume_ownership};
pub use shell::{lotus_command, forge_command, cast_command, lotus_wallet_command};