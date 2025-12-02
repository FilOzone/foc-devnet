//! Docker status reporting utilities.
//!
//! This module provides functions for checking Docker container status,
//! port mappings, uptime calculations, and system information.

use crate::docker::core::{container_is_running, docker_command};
use chrono::{DateTime, Utc};

/// Get list of running Docker containers with foc- prefix.
pub fn get_running_foc_containers() -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
pub fn get_container_start_time(
    container_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = docker_command(&[
        "inspect",
        container_name,
        "--format",
        "{{.State.StartedAt}}",
    ])?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get port mappings for a Docker container.
pub fn get_container_ports(
    container_name: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    docker_command(&["port", container_name])
}

/// Get the running time information for foc- containers.
pub fn get_foc_containers_running_time() -> Result<std::process::Output, Box<dyn std::error::Error>>
{
    docker_command(&["ps", "--filter", "name=foc-", "--format", "{{.RunningFor}}"])
}

/// Get uptime for a running container.
pub fn get_container_uptime(container_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    if !container_is_running(container_name)? {
        return Ok("Not running".to_string());
    }

    let start_time_str = get_container_start_time(container_name)?;
    if start_time_str.is_empty() || start_time_str == "null" {
        return Ok("Unknown".to_string());
    }

    let start_time = DateTime::parse_from_rfc3339(&start_time_str)
        .map_err(|e| format!("Failed to parse start time '{}': {}", start_time_str, e))?
        .with_timezone(&Utc);

    let now = Utc::now();
    let duration = now.signed_duration_since(start_time);

    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    if days > 0 {
        Ok(format!("{}d {}h {}m {}s", days, hours, minutes, seconds))
    } else if hours > 0 {
        Ok(format!("{}h {}m {}s", hours, minutes, seconds))
    } else if minutes > 0 {
        Ok(format!("{}m {}s", minutes, seconds))
    } else {
        Ok(format!("{}s", seconds))
    }
}

/// Parse container running time from Docker ps output.
/// Parse container running time into a compact format.
pub fn parse_container_running_time(
    running_time: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    if running_time.trim().is_empty() || running_time == "<no value>" {
        return Ok("Not running".to_string());
    }

    // Docker formats: "2 hours ago", "3 minutes ago", "About a minute ago", etc.
    let parts: Vec<&str> = running_time.split_whitespace().collect();

    if parts.len() >= 3 && parts[1] == "ago" {
        parse_standard_time_format(&parts)
    } else if parts.len() >= 4 && parts[0] == "About" && parts[1] == "a" {
        parse_about_time_format(&parts)
    } else {
        Ok(running_time.to_string())
    }
}

/// Parse standard Docker time format like "2 hours ago".
fn parse_standard_time_format(parts: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let number = parts[0].parse::<i64>().unwrap_or(0);
    let unit = parts[2].trim_end_matches('s'); // Remove plural 's'

    let compact_unit = match unit {
        "second" => "s",
        "minute" => "m",
        "hour" => "h",
        "day" => "d",
        "week" => "w",
        "month" => "mo",
        "year" => "y",
        _ => return Ok(parts[0..3].join(" ")),
    };

    Ok(format!("{}{}", number, compact_unit))
}

/// Parse "About a minute ago" style format.
fn parse_about_time_format(parts: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let unit = parts[3].trim_end_matches('s');
    let compact_unit = match unit {
        "second" => "1s",
        "minute" => "1m",
        "hour" => "1h",
        "day" => "1d",
        _ => "1m",
    };
    Ok(compact_unit.to_string())
}

/// Get the system start time (oldest running container start time).
pub fn get_system_start_time(
) -> Result<Option<chrono::DateTime<chrono::Utc>>, Box<dyn std::error::Error>> {
    let containers = get_running_foc_containers()?;

    if containers.is_empty() {
        return Ok(None);
    }

    let mut oldest_time: Option<chrono::DateTime<chrono::Utc>> = None;

    for container in containers {
        let start_time_str = get_container_start_time(&container)?;
        if let Ok(start_time) = chrono::DateTime::parse_from_rfc3339(&start_time_str) {
            let utc_time = start_time.with_timezone(&chrono::Utc);
            if oldest_time.is_none() || utc_time < oldest_time.unwrap() {
                oldest_time = Some(utc_time);
            }
        }
    }

    Ok(oldest_time)
}
