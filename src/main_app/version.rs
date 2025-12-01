//! Version information display.
//!
//! This module handles displaying version and build information.

/// Execute the version command
pub fn handle_version() -> Result<(), Box<dyn std::error::Error>> {
    // Version information is read-only, no poison protection needed
    println!("foc-localnet {}", env!("CARGO_PKG_VERSION"));
    println!("Commit: {}", env!("GIT_COMMIT"));
    println!("Branch: {}", env!("GIT_BRANCH"));

    // Calculate relative time
    let build_timestamp: i64 = env!("BUILD_TIMESTAMP").parse().unwrap_or(0);
    let now = chrono::Utc::now().timestamp();
    let diff_seconds = now - build_timestamp;

    let relative_time = if diff_seconds < 60 {
        format!("({} seconds ago)", diff_seconds)
    } else if diff_seconds < 3600 {
        format!("({} minutes ago)", diff_seconds / 60)
    } else if diff_seconds < 86400 {
        format!("({} hours ago)", diff_seconds / 3600)
    } else {
        format!("({} days ago)", diff_seconds / 86400)
    };

    println!("Built (UTC): {} {}", env!("BUILD_TIME_UTC"), relative_time);
    println!("Built (Local): {}", env!("BUILD_TIME_LOCAL"));
    Ok(())
}
