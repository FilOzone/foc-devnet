//! Docker container status utilities.
//!
//! This module provides utilities for checking container status and uptime.

use chrono::{DateTime, Utc};
use crossterm::style::Stylize;
use std::process::Command;

/// Get list of running Docker containers with foc- prefix.
///
/// This function queries Docker for all running containers whose names start with "foc-".
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::docker::get_running_containers;
///
/// let containers = get_running_containers().unwrap();
/// for container in containers {
///     println!("Running: {}", container);
/// }
/// ```
///
/// # Errors
///
/// Returns an error if the Docker command fails to execute.
pub fn get_running_containers() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=foc-", "--format", "{{.Names}}"])
        .output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let containers: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(containers)
}

/// Get uptime for a running container.
///
/// This function retrieves the container's start time from Docker and calculates
/// how long it has been running.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::docker::get_container_uptime;
///
/// let uptime = get_container_uptime("foc-lotus").unwrap();
/// println!("Container uptime: {}", uptime);
/// ```
///
/// # Parameters
///
/// * `container_name` - The name of the Docker container
///
/// # Errors
///
/// Returns an error if the Docker command fails or if the timestamp cannot be parsed.
pub fn get_container_uptime(container_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args([
            "inspect",
            container_name,
            "--format",
            "{{.State.StartedAt}}",
        ])
        .output()?;

    if !output.status.success() {
        return Ok("Unknown".dark_grey().to_string());
    }

    let started_at_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse the datetime string from Docker
    if let Ok(started_at) = DateTime::parse_from_rfc3339(&started_at_str) {
        let started_at_utc: DateTime<Utc> = started_at.with_timezone(&Utc);
        let now = Utc::now();
        let duration = now.signed_duration_since(started_at_utc);

        Ok(format_duration(duration).green().to_string())
    } else {
        Ok("Unknown".dark_grey().to_string())
    }
}

/// Format a duration into a human-readable string.
///
/// # Parameters
///
/// * `duration` - The duration to format
///
/// # Returns
///
/// A formatted string representing the duration.
fn format_duration(duration: chrono::Duration) -> String {
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
