//! # System Uptime
//!
//! This module handles the display of system uptime information for foc-localnet.
//!
//! It provides functionality to:
//! - Calculate total system uptime based on container start times
//! - Display formatted uptime strings
//! - Handle cases where the system is not running

use chrono::{DateTime, Utc};
use std::process::Command;
use tracing::{info, warn};

use super::utils::format_duration;
use crate::docker::status::{get_container_start_time, get_running_foc_containers};

/// Get the current lotus chain block height.
///
/// This function queries the lotus node to get the current block height of the chain.
/// Returns `None` if the lotus container is not running or if the command fails.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::uptime::get_lotus_block_height;
///
/// if let Some(height) = get_lotus_block_height() {
///     println!("Current block height: {}", height);
/// }
/// ```
fn get_lotus_block_height() -> Option<u64> {
    let output = Command::new("docker")
        .args([
            "exec",
            "foc-lotus",
            "/usr/local/bin/lotus-bins/lotus",
            "chain",
            "list",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?;

    // Parse the block height from the first line (format: "HEIGHT: (timestamp) [ ... ]")
    let height_str = first_line.split(':').next()?.trim();
    height_str.parse::<u64>().ok()
}

/// Get the current CPU usage for foc- containers as a percentage.
///
/// This function uses docker stats to get the CPU usage of all running foc- containers.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::uptime::get_containers_cpu_usage;
///
/// if let Some(cpu) = get_containers_cpu_usage() {
///     println!("Containers CPU usage: {:.1}%", cpu);
/// }
/// ```
fn get_containers_cpu_usage() -> Option<f32> {
    let output = Command::new("docker")
        .args([
            "stats",
            "--no-stream",
            "--format",
            "{{.Name}}\t{{.CPUPerc}}",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut total_cpu = 0.0;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 && parts[0].starts_with("foc-") {
            // Parse CPU percentage (remove % sign)
            if let Some(cpu_str) = parts[1].strip_suffix('%') {
                if let Ok(cpu) = cpu_str.parse::<f32>() {
                    total_cpu += cpu;
                }
            }
        }
    }

    Some(total_cpu)
}

/// Get the current memory usage for foc- containers.
///
/// This function returns a tuple of (used_memory_gb, total_limit_gb).
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::uptime::get_containers_memory_usage;
///
/// if let Some((used, limit)) = get_containers_memory_usage() {
///     println!("Containers memory: {:.1}GB / {:.1}GB", used, limit);
/// }
/// ```
fn get_containers_memory_usage() -> Option<(f64, f64)> {
    let output = Command::new("docker")
        .args([
            "stats",
            "--no-stream",
            "--format",
            "{{.Name}}\t{{.MemUsage}}",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut total_used_gb = 0.0;
    let mut total_limit_gb = 0.0;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 && parts[0].starts_with("foc-") {
            // Parse memory usage (format: "647.5MiB / 62.71GiB")
            let mem_parts: Vec<&str> = parts[1].split(" / ").collect();
            if mem_parts.len() == 2 {
                // Parse used memory
                if let Some(used_str) = parse_memory_value(mem_parts[0]) {
                    total_used_gb += used_str;
                }
                // Parse limit (only once, assuming all containers have same limit)
                if total_limit_gb == 0.0 {
                    total_limit_gb = parse_memory_value(mem_parts[1]).unwrap_or(0.0);
                }
            }
        }
    }

    if total_used_gb > 0.0 {
        Some((total_used_gb, total_limit_gb))
    } else {
        None
    }
}

/// Parse memory value from docker stats format (e.g., "647.5MiB" -> 0.6475 GB)
fn parse_memory_value(mem_str: &str) -> Option<f64> {
    let mem_str = mem_str.trim();
    if let Some(mib_pos) = mem_str.find("MiB") {
        let value_str = &mem_str[..mib_pos];
        value_str.parse::<f64>().ok().map(|v| v / 1024.0) // Convert MiB to GiB
    } else if let Some(gib_pos) = mem_str.find("GiB") {
        let value_str = &mem_str[..gib_pos];
        value_str.parse::<f64>().ok()
    } else {
        None
    }
}

/// Print uptime information if system is running.
///
/// This function displays the total uptime of the foc-localnet system by finding
/// the oldest running container and calculating how long the system has been running.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::uptime::print_uptime;
///
/// print_uptime().expect("Failed to print uptime");
/// ```
///
/// # Errors
///
/// Returns an error if Docker commands fail.
pub fn print_uptime() -> Result<(), Box<dyn std::error::Error>> {
    info!("System Uptime");

    let containers = get_running_foc_containers()?;

    if containers.is_empty() {
        info!("System is not running");
        return Ok(());
    }

    // Get the oldest container start time as system start time
    let mut oldest_start_time: Option<DateTime<Utc>> = None;

    for container in &containers {
        if let Ok(start_time_str) = get_container_start_time(container) {
            if let Ok(start_time) = DateTime::parse_from_rfc3339(&start_time_str) {
                let start_time_utc = start_time.with_timezone(&Utc);
                match oldest_start_time {
                    None => oldest_start_time = Some(start_time_utc),
                    Some(oldest) if start_time_utc < oldest => {
                        oldest_start_time = Some(start_time_utc)
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(start_time) = oldest_start_time {
        let now = Utc::now();
        let uptime = now.signed_duration_since(start_time);

        let total_seconds = uptime.num_seconds();
        let uptime_str = format_duration(total_seconds);

        info!("System uptime: {}", uptime_str);

        // Try to get lotus block height if chain is running
        if let Some(block_height) = get_lotus_block_height() {
            info!("Chain height (lotus): {}", block_height);
        }

        // Display CPU usage
        if let Some(cpu_usage) = get_containers_cpu_usage() {
            info!("Containers CPU usage: {:.1}%", cpu_usage);
        }

        // Display RAM usage
        if let Some((used_ram, total_ram)) = get_containers_memory_usage() {
            info!(
                "Containers RAM usage: {:.1}GB / {:.1}GB",
                used_ram, total_ram
            );
        }
    } else {
        warn!("Unable to determine uptime");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_uptime() {
        // This test verifies that the function doesn't panic
        let result = print_uptime();
        // We expect this to work even if no containers are running
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_lotus_block_height() {
        // This test verifies that the function doesn't panic
        let _height = get_lotus_block_height();
        // We don't assert anything as the result depends on whether lotus is running
    }
}
