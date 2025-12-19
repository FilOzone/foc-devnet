//! # Status Utilities
//!
//! Common utility functions used across the status module for formatting and data processing.
//!
//! This module provides shared functionality for:
//! - Size formatting (bytes to human-readable)
//! - Time duration formatting
//! - Common data structures

/// Format a size in bytes to human readable format.
///
/// # Examples
///
/// ```rust
/// use foc_localnet::commands::status::utils::format_size;
///
/// assert_eq!(format_size(1024), "1.0 KB");
/// assert_eq!(format_size(1048576), "1.0 MB");
/// assert_eq!(format_size(500), "500 B");
/// ```
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Format a time duration in seconds to a human-readable string.
///
/// # Examples
///
/// ```rust
/// use foc_localnet::commands::status::utils::format_duration;
///
/// assert_eq!(format_duration(3661), "1h 1m 1s");
/// assert_eq!(format_duration(86461), "1d 0h 1m 1s");
/// ```
pub fn format_duration(total_seconds: i64) -> String {
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Format a time duration as a relative "ago" string.
///
/// # Examples
///
/// ```rust
/// use foc_localnet::commands::status::utils::format_time_ago;
/// use chrono::{Duration, Utc};
///
/// let duration = Duration::hours(2) + Duration::minutes(30);
/// assert_eq!(format_time_ago(duration), "2h 30m ago");
/// ```
pub fn format_time_ago(duration: chrono::Duration) -> String {
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;

    if days > 0 {
        format!("{}d {}h ago", days, hours)
    } else if hours > 0 {
        format!("{}h {}m ago", hours, minutes)
    } else if minutes > 0 {
        format!("{}m ago", minutes)
    } else {
        "just now".to_string()
    }
}

/// Get the size of a directory in bytes using `du` command.
///
/// This function uses the system `du` command to calculate directory size.
/// If the directory doesn't exist or the command fails, returns 0.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::utils::get_directory_size;
/// use std::path::Path;
///
/// let size = get_directory_size(Path::new("/tmp")).unwrap();
/// println!("Directory size: {} bytes", size);
/// ```
///
/// # Errors
///
/// Returns an error if the `du` command execution fails.
pub fn get_directory_size(path: &std::path::Path) -> Result<u64, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(0);
    }

    let output = std::process::Command::new("du")
        .args(["-sb", path.to_str().unwrap_or(".")])
        .output()?;

    if !output.status.success() {
        return Ok(0);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = stdout.lines().next() {
        if let Some(size_str) = line.split_whitespace().next() {
            if let Ok(size) = size_str.parse::<u64>() {
                return Ok(size);
            }
        }
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(59), "59s");
        assert_eq!(format_duration(60), "1m 0s");
        assert_eq!(format_duration(3661), "1h 1m 1s");
        assert_eq!(format_duration(86400), "1d 0h 0m 0s");
    }
}
