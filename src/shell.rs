//! Shell command abstractions.
//!
//! This module provides high-level abstractions for shell commands used throughout
//! the foc-localnet codebase. It centralizes command execution logic and provides
//! consistent error handling and logging.
//!
//! Instead of scattering `Command::new()` calls throughout the codebase, all shell
//! operations should go through functions in this module.

use std::error::Error;
use std::process::{Command, Output};

/// Execute a shell command and return its output.
///
/// # Arguments
/// * `program` - The program to execute (e.g., "docker", "lotus")
/// * `args` - Command line arguments
///
/// # Returns
/// The command output on success, or an error on failure.
///
/// # Examples
/// ```rust
/// use crate::shell::run_command;
///
/// let output = run_command("docker", &["ps"])?;
/// println!("Containers: {}", String::from_utf8_lossy(&output.stdout));
/// ```
pub fn run_command(program: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Command failed: {} {} -> {}",
            program,
            args.join(" "),
            stderr
        )
        .into());
    }
    Ok(output)
}

/// Execute a docker command.
///
/// # Arguments
/// * `args` - Docker command arguments (without the 'docker' prefix)
///
/// # Returns
/// The command output on success.
///
/// # Examples
/// ```rust
/// use crate::shell::docker_command;
///
/// let output = docker_command(&["ps", "--format", "{{.Names}}"])?;
/// ```
pub fn docker_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    run_command("docker", args)
}

/// Check if a docker container exists.
///
/// # Arguments
/// * `name` - Container name
///
/// # Returns
/// true if container exists, false otherwise.
pub fn docker_container_exists(name: &str) -> Result<bool, Box<dyn Error>> {
    let output = docker_command(&[
        "ps",
        "-a",
        "--filter",
        &format!("name=^{}$", name),
        "--format",
        "{{.Names}}",
    ])?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(name))
}

/// Check if a docker container is running.
///
/// # Arguments
/// * `name` - Container name
///
/// # Returns
/// true if container is running, false otherwise.
pub fn docker_container_is_running(name: &str) -> Result<bool, Box<dyn Error>> {
    let output = docker_command(&[
        "ps",
        "--filter",
        &format!("name=^{}$", name),
        "--format",
        "{{.Names}}",
    ])?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(name))
}

/// Stop a docker container.
///
/// # Arguments
/// * `name` - Container name
pub fn docker_stop_container(name: &str) -> Result<(), Box<dyn Error>> {
    docker_command(&["stop", name])?;
    Ok(())
}

/// Remove a docker container.
///
/// # Arguments
/// * `name` - Container name
pub fn docker_remove_container(name: &str) -> Result<(), Box<dyn Error>> {
    docker_command(&["rm", name])?;
    Ok(())
}

/// Execute a command inside a docker container.
///
/// # Arguments
/// * `container` - Container name
/// * `command` - Command to execute inside container
/// * `args` - Command arguments
///
/// # Returns
/// The command output.
///
/// # Examples
/// ```rust
/// use crate::shell::docker_exec;
///
/// let output = docker_exec("foc-lotus", "lotus", &["wallet", "list"])?;
/// ```
pub fn docker_exec(
    container: &str,
    command: &str,
    args: &[&str],
) -> Result<Output, Box<dyn Error>> {
    let mut exec_args = vec!["exec", container, command];
    exec_args.extend_from_slice(args);
    docker_command(&exec_args)
}

/// Check if a docker image exists.
///
/// # Arguments
/// * `image_name` - Image name
///
/// # Returns
/// true if image exists, false otherwise.
pub fn docker_image_exists(image_name: &str) -> Result<bool, Box<dyn Error>> {
    let result = docker_command(&["image", "inspect", image_name]);
    match result {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Run a docker container.
///
/// # Arguments
/// * `args` - Docker run arguments
pub fn docker_run(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut run_args = vec!["run"];
    run_args.extend_from_slice(args);
    docker_command(&run_args)
}

/// Execute a lotus command inside the foc-lotus container.
///
/// # Arguments
/// * `args` - Lotus command arguments
///
/// # Returns
/// The command output.
///
/// # Examples
/// ```rust
/// use crate::shell::lotus_command;
///
/// let output = lotus_command(&["wallet", "list"])?;
/// ```
pub fn lotus_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    docker_exec("foc-lotus", "/usr/local/bin/lotus-bins/lotus", args)
}

/// Execute a lotus-miner command inside the foc-lotus-miner container.
///
/// # Arguments
/// * `args` - Lotus-miner command arguments
///
/// # Returns
/// The command output.
pub fn lotus_miner_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    docker_exec(
        "foc-lotus-miner",
        "/usr/local/bin/lotus-bins/lotus-miner",
        args,
    )
}

/// Execute a forge command inside the foc-builder container.
///
/// # Arguments
/// * `args` - Forge command arguments
///
/// # Returns
/// The command output.
pub fn forge_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    docker_exec("foc-builder", "forge", args)
}

/// Execute a cast command inside the foc-builder container.
///
/// # Arguments
/// * `args` - Cast command arguments
///
/// # Returns
/// The command output.
pub fn cast_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    docker_exec("foc-builder", "cast", args)
}
