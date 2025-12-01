//! Docker utility functions for foc-localnet
//!
//! This module contains shared utility functions for Docker operations
//! used across different commands, particularly container management.
//!
//! This is the central location for all basic Docker operations to avoid
//! duplication across the codebase.

use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Check if a port is available (not in use)
pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}

/// Check if a Docker image exists
pub fn image_exists(image_name: &str) -> bool {
    Command::new("docker")
        .args(["image", "inspect", image_name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if a container with the given name exists
pub fn container_exists(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name=^{}$", name),
            "--format",
            "{{.Names}}",
        ])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(name))
}

/// Check if a container is running
pub fn container_is_running(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            &format!("name=^{}$", name),
            "--format",
            "{{.Names}}",
        ])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .contains(name))
}

/// Stop a container if it's running
pub fn stop_container(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if container_is_running(name)? {
        Command::new("docker").args(["stop", name]).output()?;
    }
    Ok(())
}

/// Remove a container if it exists
pub fn remove_container(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if container_exists(name)? {
        Command::new("docker").args(["rm", name]).output()?;
    }
    Ok(())
}

/// Stop and remove a container if it exists
pub fn stop_and_remove_container(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    stop_container(name)?;
    remove_container(name)?;
    Ok(())
}

/// Execute a command inside a running container
pub fn exec_in_container(
    container: &str,
    command: &str,
    args: &[&str],
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let mut exec_args = vec!["exec", container, command];
    exec_args.extend_from_slice(args);

    Command::new("docker")
        .args(&exec_args)
        .output()
        .map_err(|e| e.into())
}

/// Run a Docker container
pub fn run_container(args: &[&str]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let mut run_args = vec!["run"];
    run_args.extend_from_slice(args);

    Command::new("docker")
        .args(&run_args)
        .output()
        .map_err(|e| e.into())
}

/// Wait for a port to be accepting connections
pub fn wait_for_port(port: u16, timeout_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    loop {
        if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return Ok(());
        }

        if start.elapsed().as_secs() > timeout_secs {
            return Err(format!("Timeout waiting for port {} to be ready", port).into());
        }

        thread::sleep(Duration::from_millis(100)); // Use a reasonable default interval
    }
}
