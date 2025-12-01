//! Shell command abstractions.
//!
//! This module provides high-level abstractions for shell commands used throughout
//! the foc-localnet codebase. It centralizes command execution logic and provides
//! consistent error handling and logging.
//!
//! Instead of scattering `Command::new()` calls throughout the codebase, all shell
//! operations should go through functions in this module.

use crate::docker;
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
    docker::container_exists(name)
}

/// Check if a docker container is running.
///
/// # Arguments
/// * `name` - Container name
///
/// # Returns
/// true if container is running, false otherwise.
pub fn docker_container_is_running(name: &str) -> Result<bool, Box<dyn Error>> {
    docker::container_is_running(name)
}

/// Stop a docker container.
///
/// # Arguments
/// * `name` - Container name
pub fn docker_stop_container(name: &str) -> Result<(), Box<dyn Error>> {
    docker::stop_container(name)
}

/// Remove a Docker container.
///
/// # Arguments
/// * `name` - Container name
///
/// # Returns
/// The command output.
pub fn docker_remove_container(name: &str) -> Result<Output, Box<dyn Error>> {
    docker_command(&["rm", name])
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
    docker::exec_in_container(container, command, args)
}

/// Check if a docker image exists.
///
/// # Arguments
/// * `image_name` - Image name
///
/// # Returns
/// true if image exists, false otherwise.
pub fn docker_image_exists(image_name: &str) -> Result<bool, Box<dyn Error>> {
    Ok(docker::image_exists(image_name))
}

/// Check if a Docker image exists locally.
///
/// # Arguments
/// * `image_tag` - The image tag to check (e.g., "foc-localnet-builder:latest")
///
/// # Returns
/// true if the image exists, false otherwise.
pub fn image_exists(image_tag: &str) -> Result<bool, Box<dyn Error>> {
    let output = docker_command(&["images", "--format", "{{.Repository}}:{{.Tag}}"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().any(|line| line == image_tag))
}

/// Run a docker container.
///
/// # Arguments
/// * `args` - Docker run arguments
pub fn docker_run(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    docker::run_container(args)
}

/// Build a Docker image from a Dockerfile.
///
/// # Arguments
/// * `dockerfile_path` - Path to the Dockerfile
/// * `image_tag` - Tag for the built image
/// * `context_dir` - Build context directory
///
/// # Returns
/// The command output.
pub fn build_docker_image(
    dockerfile_path: &str,
    image_tag: &str,
    context_dir: &str,
) -> Result<Output, Box<dyn Error>> {
    docker_command(&[
        "build",
        "--progress",
        "tty",
        "-f",
        dockerfile_path,
        "-t",
        image_tag,
        context_dir,
    ])
}

/// Build a Docker image with custom build arguments.
///
/// # Arguments
/// * `dockerfile_path` - Path to the Dockerfile
/// * `image_tag` - Tag for the built image
/// * `build_context` - Build context directory
/// * `build_args` - Additional build arguments
///
/// # Returns
/// The command output.
pub fn build_docker_image_with_args(
    dockerfile_path: &str,
    image_tag: &str,
    build_context: &str,
    build_args: &[(&str, &str)],
) -> Result<Output, Box<dyn Error>> {
    let mut args = vec!["build", "--progress", "plain"];
    
    // Format build args first to avoid temporary value issues
    let formatted_args: Vec<String> = build_args
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect();
    
    for formatted_arg in &formatted_args {
        args.push("--build-arg");
        args.push(formatted_arg);
    }
    
    args.extend_from_slice(&["--file", dockerfile_path, "--tag", image_tag, build_context]);
    
    docker_command(&args)
}/// Execute a lotus command inside the foc-lotus container.
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

/// Check if a specific docker container is running by name.
///
/// # Arguments
/// * `container_name` - Exact container name to check
///
/// # Returns
/// true if the container is running, false otherwise.
///
/// # Examples
/// ```rust
/// use crate::shell::is_container_running;
///
/// if is_container_running("foc-lotus")? {
///     println!("Lotus is running");
/// }
/// ```
pub fn is_container_running(container_name: &str) -> Result<bool, Box<dyn Error>> {
    let output = docker_command(&[
        "ps",
        "--filter",
        &format!("name=^{}$", container_name),
        "--format",
        "{{.Names}}",
    ])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().contains(container_name))
}

/// Execute a lotus wallet command.
///
/// # Arguments
/// * `args` - Wallet command arguments (without 'wallet' prefix)
///
/// # Returns
/// The command output.
///
/// # Examples
/// ```rust
/// use crate::shell::lotus_wallet_command;
///
/// let output = lotus_wallet_command(&["list"])?;
/// ```
pub fn lotus_wallet_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut full_args = vec!["wallet"];
    full_args.extend_from_slice(args);
    lotus_command(&full_args)
}

/// Execute a lotus evm command.
///
/// # Arguments
/// * `args` - EVM command arguments (without 'evm' prefix)
///
/// # Returns
/// The command output.
pub fn lotus_evm_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut full_args = vec!["evm"];
    full_args.extend_from_slice(args);
    lotus_command(&full_args)
}

/// Execute a lotus send command to transfer FIL.
///
/// # Arguments
/// * `from` - Sender address
/// * `to` - Recipient address
/// * `amount` - Amount to send
///
/// # Returns
/// The command output.
pub fn lotus_send_fil(from: &str, to: &str, amount: &str) -> Result<Output, Box<dyn Error>> {
    lotus_command(&["send", "--from", from, to, amount])
}

/// Create a new delegated (f4) address for FEVM operations.
///
/// # Returns
/// The new address on success.
pub fn lotus_create_delegated_address() -> Result<String, Box<dyn Error>> {
    let output = lotus_wallet_command(&["new", "delegated"])?;
    let address = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(address)
}

/// Import a key into lotus wallet from a hex-encoded keyinfo file.
///
/// # Arguments
/// * `keyinfo_path` - Path to the hex-encoded keyinfo file (container path)
///
/// # Returns
/// The imported address on success.
pub fn lotus_import_key(keyinfo_path: &str) -> Result<String, Box<dyn Error>> {
    let output = lotus_wallet_command(&["import", keyinfo_path])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let address = stdout
        .lines()
        .find(|line| line.starts_with("imported key"))
        .and_then(|line| line.split_whitespace().nth(2))
        .ok_or("Failed to extract imported address")?
        .to_string();
    Ok(address)
}

/// Export a private key from lotus wallet.
///
/// # Arguments
/// * `address` - Address to export
///
/// # Returns
/// The exported key data.
pub fn lotus_export_key(address: &str) -> Result<Output, Box<dyn Error>> {
    lotus_wallet_command(&["export", address])
}

/// Get the Ethereum address corresponding to an f4 address.
///
/// # Arguments
/// * `f4_address` - The f4 address
///
/// # Returns
/// The corresponding Ethereum address.
pub fn lotus_get_eth_address(f4_address: &str) -> Result<String, Box<dyn Error>> {
    let output = lotus_evm_command(&["stat", f4_address])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let eth_addr = stdout
        .lines()
        .find(|line| line.contains("Eth address:"))
        .and_then(|line| line.split_whitespace().nth(2))
        .ok_or("Failed to extract Ethereum address")?
        .to_string();
    Ok(eth_addr)
}

/// Run a docker container with host networking.
///
/// # Arguments
/// * `image` - Container image
/// * `args` - Additional docker run arguments
///
/// # Returns
/// The command output.
pub fn docker_run_host_network(image: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut full_args = vec!["run", "--rm", "--network", "host"];
    full_args.extend_from_slice(args);
    full_args.extend_from_slice(&["-i", image]);
    docker_command(&full_args)
}

/// Run a docker container with volume mounts.
///
/// # Arguments
/// * `image` - Container image
/// * `volumes` - Volume mount specifications in format "host_path:container_path"
/// * `args` - Additional docker run arguments
///
/// # Returns
/// The command output.
pub fn docker_run_with_volumes(
    image: &str,
    volumes: &[&str],
    args: &[&str],
) -> Result<Output, Box<dyn Error>> {
    let mut full_args = vec!["run", "--rm"];
    for volume in volumes {
        full_args.push("-v");
        full_args.push(volume);
    }
    full_args.extend_from_slice(args);
    full_args.push(image);
    docker_command(&full_args)
}

/// Execute a bash command inside the foc-builder container.
///
/// # Arguments
/// * `command` - Bash command to execute
///
/// # Returns
/// The command output.
pub fn foc_builder_bash_command(command: &str) -> Result<Output, Box<dyn Error>> {
    docker_exec("foc-builder", "bash", &["-c", command])
}

/// Execute a forge build command in the foc-builder container.
///
/// # Arguments
/// * `working_dir` - Working directory inside container
///
/// # Returns
/// The command output.
pub fn forge_build_in_container(working_dir: &str) -> Result<Output, Box<dyn Error>> {
    let command = format!("cd {} && forge build", working_dir);
    foc_builder_bash_command(&command)
}

/// Execute a forge script command in the foc-builder container.
///
/// # Arguments
/// * `script_path` - Path to the script file
/// * `rpc_url` - RPC URL for deployment
/// * `private_key` - Private key for signing
/// * `extra_args` - Additional forge script arguments
///
/// # Returns
/// The command output.
pub fn forge_script_deploy(
    script_path: &str,
    rpc_url: &str,
    private_key: &str,
    extra_args: &[&str],
) -> Result<Output, Box<dyn Error>> {
    let mut args = vec![
        "script", script_path,
        "--rpc-url", rpc_url,
        "--private-key", private_key,
        "--broadcast",
    ];
    args.extend_from_slice(extra_args);
    forge_command(&args)
}

/// Get list of running Docker containers with foc- prefix.
///
/// # Returns
/// A vector of container names that are currently running and start with "foc-".
pub fn get_running_foc_containers() -> Result<Vec<String>, Box<dyn Error>> {
    let output = docker_command(&["ps", "--filter", "name=foc-", "--format", "{{.Names}}"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let containers: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    Ok(containers)
}

/// Get the start time of a Docker container.
///
/// # Arguments
/// * `container_name` - Name of the container
///
/// # Returns
/// The container's start time as a string, or "Unknown" if not found.
pub fn get_container_start_time(container_name: &str) -> Result<String, Box<dyn Error>> {
    let output = docker_command(&[
        "inspect",
        container_name,
        "--format",
        "{{.State.StartedAt}}",
    ])?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get port mappings for a Docker container.
///
/// # Arguments
/// * `container_name` - Name of the container
///
/// # Returns
/// The output of `docker port` command.
pub fn get_container_ports(container_name: &str) -> Result<Output, Box<dyn Error>> {
    docker_command(&["port", container_name])
}

/// Get the running time information for foc- containers.
///
/// # Returns
/// The output of `docker ps --filter name=foc- --format {{.RunningFor}}`.
pub fn get_foc_containers_running_time() -> Result<Output, Box<dyn Error>> {
    docker_command(&["ps", "--filter", "name=foc-", "--format", "{{.RunningFor}}"])
}

/// Get the current user's UID.
///
/// # Returns
/// The current user's UID as a string.
pub fn get_current_uid() -> Result<String, Box<dyn Error>> {
    let output = Command::new("id")
        .arg("-u")
        .output()?;
    
    if !output.status.success() {
        return Err(format!("Failed to get current UID: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Get the current user's GID.
///
/// # Returns
/// The current user's GID as a string.
pub fn get_current_gid() -> Result<String, Box<dyn Error>> {
    let output = Command::new("id")
        .arg("-g")
        .output()?;
    
    if !output.status.success() {
        return Err(format!("Failed to get current GID: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Execute a chown command to change file ownership.
///
/// # Arguments
/// * `args` - Chown command arguments
///
/// # Returns
/// The command output.
pub fn chown_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new("chown");
    command.args(args);
    let output = command.output()?;
    Ok(output)
}

/// Create a Docker container without starting it.
///
/// # Arguments
/// * `container_name` - Name for the container
/// * `image_tag` - Docker image to use
/// * `command` - Command to run in the container
///
/// # Returns
/// The command output.
pub fn docker_create_container(
    container_name: &str,
    image_tag: &str,
    command: &str,
) -> Result<Output, Box<dyn Error>> {
    docker_command(&["create", "--name", container_name, image_tag, command])
}

/// Copy files from a Docker container to the host.
///
/// # Arguments
/// * `container_name` - Name of the source container
/// * `container_path` - Path inside the container
/// * `host_path` - Host destination path
///
/// # Returns
/// The command output.
pub fn docker_copy_from_container(
    container_name: &str,
    container_path: &str,
    host_path: &str,
) -> Result<Output, Box<dyn Error>> {
    docker_command(&["cp", &format!("{}:{}", container_name, container_path), host_path])
}
