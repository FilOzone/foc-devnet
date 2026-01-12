//! Version information utilities.
//!
//! This module provides utilities for writing version information to files.

use chrono::Utc;
use std::error::Error;
use std::fs;
use std::path::Path;

/// Version information structure containing build-time constants.
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: &'static str,
    pub commit: &'static str,
    pub branch: &'static str,
    pub dirty: &'static str,
    pub build_time_utc: &'static str,
    pub build_time_local: &'static str,
    pub build_timestamp: i64,
}

impl VersionInfo {
    /// Create VersionInfo from compile-time environment variables.
    pub fn from_env() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            commit: env!("GIT_COMMIT"),
            branch: env!("GIT_BRANCH"),
            dirty: env!("GIT_DIRTY"),
            build_time_utc: env!("BUILD_TIME_UTC"),
            build_time_local: env!("BUILD_TIME_LOCAL"),
            build_timestamp: env!("BUILD_TIMESTAMP").parse().unwrap_or(0),
        }
    }
}

/// Write version information to a version.txt file.
///
/// Creates a version.txt file in the specified directory containing:
/// - foc-devnet version
/// - Git commit hash (with dirty indicator if uncommitted changes exist)
/// - Git branch
/// - Build timestamps
///
/// # Arguments
/// * `dir` - Directory where version.txt will be created
/// * `version_info` - Version information to write
///
/// # Example
/// ```no_run
/// use std::path::PathBuf;
/// let version_info = VersionInfo::from_env();
/// write_version_file(&PathBuf::from("/tmp/logs"), &version_info).unwrap();
/// ```
pub fn write_version_file(dir: &Path, version_info: &VersionInfo) -> Result<(), Box<dyn Error>> {
    let version_file = dir.join("version.txt");

    let now = Utc::now().timestamp();
    let diff_seconds = now - version_info.build_timestamp;

    let relative_time = calculate_relative_time(diff_seconds);
    let dirty_suffix = if version_info.dirty.is_empty() {
        ""
    } else {
        "-dirty"
    };

    let content = format!(
        "foc-devnet {}\n\
         Commit: {}{}\n\
         Branch: {}\n\
         Built (UTC): {} {}\n\
         Built (Local): {}\n",
        version_info.version,
        version_info.commit,
        dirty_suffix,
        version_info.branch,
        version_info.build_time_utc,
        relative_time,
        version_info.build_time_local
    );

    fs::write(&version_file, content)?;

    Ok(())
}

/// Calculate relative time string from seconds difference.
fn calculate_relative_time(diff_seconds: i64) -> String {
    if diff_seconds < 60 {
        format!("({} seconds ago)", diff_seconds)
    } else if diff_seconds < 3600 {
        format!("({} minutes ago)", diff_seconds / 60)
    } else if diff_seconds < 86400 {
        format!("({} hours ago)", diff_seconds / 3600)
    } else {
        format!("({} days ago)", diff_seconds / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_time_formatting() {
        assert_eq!(calculate_relative_time(30), "(30 seconds ago)");
        assert_eq!(calculate_relative_time(120), "(2 minutes ago)");
        assert_eq!(calculate_relative_time(7200), "(2 hours ago)");
        assert_eq!(calculate_relative_time(172800), "(2 days ago)");
    }
}
