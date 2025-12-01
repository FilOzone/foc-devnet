//! Docker system time utilities.
//!
//! This module provides utilities for parsing Docker time strings and determining system start times.

use chrono::{DateTime, Utc};
use std::process::Command;

/// Get the system start time (oldest container start time).
///
/// This function finds the earliest start time among all running foc-localnet containers
/// to determine when the system was started.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::docker::get_system_start_time;
///
/// if let Some(start_time) = get_system_start_time().unwrap() {
///     println!("System started at: {}", start_time);
/// } else {
///     println!("Could not determine system start time");
/// }
/// ```
///
/// # Returns
///
/// An `Option<DateTime<Utc>>` representing the system start time, or `None` if it cannot be determined.
pub fn get_system_start_time() -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=foc-", "--format", "{{.RunningFor}}"])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut earliest_start: Option<DateTime<Utc>> = None;

    for line in stdout.lines() {
        if let Some(start_time) = parse_docker_running_for(line.trim()) {
            if earliest_start.is_none() || start_time < earliest_start.unwrap() {
                earliest_start = Some(start_time);
            }
        }
    }

    Ok(earliest_start)
}

/// Parse Docker "Running for" time string into DateTime.
///
/// This function parses the human-readable time strings that Docker provides
/// (like "2 hours", "3 minutes ago") and converts them to DateTime objects.
///
/// # Examples
///
/// ```rust
/// use foc_localnet::commands::status::docker::parse_docker_running_for;
///
/// let datetime = parse_docker_running_for("2 hours");
/// assert!(datetime.is_some());
///
/// let datetime = parse_docker_running_for("30 minutes ago");
/// assert!(datetime.is_some());
/// ```
///
/// # Parameters
///
/// * `running_for` - The Docker "Running for" string to parse
///
/// # Returns
///
/// An `Option<DateTime<Utc>>` representing the parsed time, or `None` if parsing fails.
pub fn parse_docker_running_for(running_for: &str) -> Option<DateTime<Utc>> {
    // Docker formats: "2 hours", "3 minutes ago", "About an hour ago", etc.
    // This is a simplified parser - in a real implementation you might want more robust parsing
    let now = Utc::now();

    if running_for.contains("About a minute") {
        Some(now - chrono::Duration::minutes(1))
    } else if let Some(duration) = parse_time_unit(running_for, "second") {
        Some(now - duration)
    } else if let Some(duration) = parse_time_unit(running_for, "minute") {
        Some(now - duration)
    } else if let Some(duration) = parse_time_unit(running_for, "hour") {
        Some(now - duration)
    } else if let Some(duration) = parse_time_unit(running_for, "day") {
        Some(now - duration)
    } else if let Some(duration) = parse_time_unit(running_for, "week") {
        Some(now - duration)
    } else {
        None
    }
}

/// Parse a time unit from a Docker running time string.
///
/// # Parameters
///
/// * `running_for` - The full string
/// * `unit` - The time unit to look for (e.g., "second", "minute")
///
/// # Returns
///
/// An `Option<chrono::Duration>` representing the parsed duration.
fn parse_time_unit(running_for: &str, unit: &str) -> Option<chrono::Duration> {
    if running_for.contains(unit) {
        let value: i64 = running_for.split_whitespace().next()?.parse().ok()?;
        match unit {
            "second" => Some(chrono::Duration::seconds(value)),
            "minute" => Some(chrono::Duration::minutes(value)),
            "hour" => Some(chrono::Duration::hours(value)),
            "day" => Some(chrono::Duration::days(value)),
            "week" => Some(chrono::Duration::weeks(value)),
            _ => None,
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_docker_running_for() {
        let now = Utc::now();

        let result = parse_docker_running_for("5 seconds");
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert!(parsed <= now);

        let result = parse_docker_running_for("2 hours");
        assert!(result.is_some());

        let result = parse_docker_running_for("invalid");
        assert!(result.is_none());
    }
}
