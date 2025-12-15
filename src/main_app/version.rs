//! Version information display.
//!
//! This module handles displaying version and build information.

use foc_localnet::version_info::VersionInfo;

/// Execute the version command
pub fn handle_version() -> Result<(), Box<dyn std::error::Error>> {
    // Version information is read-only, no poison protection needed
    let version_info = VersionInfo::from_env();
    let dirty_suffix = if version_info.dirty.is_empty() {
        ""
    } else {
        "-dirty"
    };

    println!("foc-localnet {}", version_info.version);
    println!("Commit: {}{}", version_info.commit, dirty_suffix);
    println!("Branch: {}", version_info.branch);

    // Calculate relative time
    let now = chrono::Utc::now().timestamp();
    let diff_seconds = now - version_info.build_timestamp;

    let relative_time = if diff_seconds < 60 {
        format!("({} seconds ago)", diff_seconds)
    } else if diff_seconds < 3600 {
        format!("({} minutes ago)", diff_seconds / 60)
    } else if diff_seconds < 86400 {
        format!("({} hours ago)", diff_seconds / 3600)
    } else {
        format!("({} days ago)", diff_seconds / 86400)
    };

    println!(
        "Built (UTC): {} {}",
        version_info.build_time_utc, relative_time
    );
    println!("Built (Local): {}", version_info.build_time_local);
    Ok(())
}
