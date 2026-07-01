//! Core Docker utilities and abstractions.
//!
//! This module provides the fundamental Docker operations and shell command abstractions
//! used throughout foc-devnet. It consolidates the functionality from the old docker.rs
//! and shell.rs modules into a single, well-organized structure.

use std::error::Error;
use std::net::TcpListener;
use std::process::{Command, Output};
use std::thread;
use std::time::Duration;

/// Execute a shell command and return its output.
///
/// # Arguments
/// * `program` - The program to execute (e.g., "docker", "lotus")
/// * `args` - Command line arguments
///
/// # Returns
/// The command output on success, or an error on failure.
pub fn run_command(program: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Truncate command display if too long
        let cmd_str = args.join(" ");
        let cmd_display = if cmd_str.len() > 200 {
            format!("{}... (truncated)", &cmd_str[..200])
        } else {
            cmd_str
        };

        return Err(format!(
            "Command failed: {} {}\nSTDOUT:\n{}\nSTDERR:\n{}",
            program, cmd_display, stdout, stderr
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
pub fn docker_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    run_command("docker", args)
}

/// Get logs from a Docker container.
///
/// Collects both stdout and stderr. Go programs (e.g. Curio)
/// write all log output to stderr, so both streams must be collected to get complete
/// logs.
///
/// # Arguments
/// * `container_name` - The name of the container to get logs from
///
/// # Returns
/// The combined stdout + stderr container logs on success.
pub fn get_container_logs(container_name: &str) -> Result<String, Box<dyn Error>> {
    let output = docker_command(&["logs", container_name])?;

    let mut logs = String::new();
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    logs.push_str(&stdout_str);
    if !stdout_str.is_empty()
        && !stderr_str.is_empty()
        && !stdout_str.ends_with('\n')
        && !stderr_str.starts_with('\n')
    {
        logs.push('\n');
    }
    logs.push_str(&stderr_str);
    Ok(logs)
}

/// Check if a port is available (not in use)
pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}

/// Check if a Docker image exists locally.
///
/// # Arguments
/// * `image_name` - The image name to check (e.g., LOTUS_DOCKER_IMAGE)
///
/// # Returns
/// true if the image exists, false otherwise.
pub fn image_exists(image_name: &str) -> Result<bool, Box<dyn Error>> {
    let output = docker_command(&["images", "--format", "{{.Repository}}:{{.Tag}}"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // image_name may be tagged ("postgres:18") -> exact match, or untagged
    // ("foc-lotus") -> match any tag.
    Ok(stdout
        .lines()
        .any(|line| line == image_name || line.starts_with(&format!("{}:", image_name))))
}

/// Check if a container with the given name exists
pub fn container_exists(name: &str) -> Result<bool, Box<dyn Error>> {
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

/// Check if a container is running
pub fn container_is_running(name: &str) -> Result<bool, Box<dyn Error>> {
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

/// Stop a container if it's running
pub fn stop_container(name: &str) -> Result<(), Box<dyn Error>> {
    if container_is_running(name)? {
        docker_command(&["stop", name])?;
    }
    Ok(())
}

/// Remove a container if it exists
pub fn remove_container(name: &str) -> Result<(), Box<dyn Error>> {
    if container_exists(name)? {
        docker_command(&["rm", name])?;
    }
    Ok(())
}

/// Stop and remove a container if it exists
pub fn stop_and_remove_container(name: &str) -> Result<(), Box<dyn Error>> {
    stop_container(name)?;
    remove_container(name)?;
    Ok(())
}

/// Execute a command inside a running container
pub fn exec_in_container(
    container: &str,
    command: &str,
    args: &[&str],
) -> Result<Output, Box<dyn Error>> {
    let mut exec_args = vec!["exec", container, command];
    exec_args.extend_from_slice(args);
    docker_command(&exec_args)
}

/// Run a Docker container
pub fn run_container(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut run_args = vec!["run"];
    run_args.extend_from_slice(args);
    docker_command(&run_args)
}

/// Create a Docker container without starting it.
pub fn create_container(
    container_name: &str,
    image_tag: &str,
    command: &str,
) -> Result<Output, Box<dyn Error>> {
    docker_command(&["create", "--name", container_name, image_tag, command])
}

/// Copy files from a Docker container to the host.
pub fn copy_from_container(
    container_name: &str,
    container_path: &str,
    host_path: &str,
) -> Result<Output, Box<dyn Error>> {
    docker_command(&[
        "cp",
        &format!("{}:{}", container_name, container_path),
        host_path,
    ])
}

/// Wait for a port to be accepting connections
pub fn wait_for_port(port: u16, timeout_secs: u64) -> Result<(), Box<dyn Error>> {
    let start = std::time::Instant::now();
    loop {
        if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return Ok(());
        }

        if start.elapsed().as_secs() > timeout_secs {
            return Err(format!("Timeout waiting for port {} to be ready", port).into());
        }

        thread::sleep(Duration::from_millis(100));
    }
}

/// Get the current user's UID.
pub fn get_current_uid() -> Result<String, Box<dyn Error>> {
    let output = Command::new("id").arg("-u").output()?;
    if !output.status.success() {
        return Err(format!(
            "Failed to get current UID: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Get the current user's GID.
pub fn get_current_gid() -> Result<String, Box<dyn Error>> {
    let output = Command::new("id").arg("-g").output()?;
    if !output.status.success() {
        return Err(format!(
            "Failed to get current GID: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Execute a chown command to change file ownership.
pub fn chown_command(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new("chown");
    command.args(args);
    command.output().map_err(|e| e.into())
}
