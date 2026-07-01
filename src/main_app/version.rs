//! Version information display.
//!
//! This module handles displaying version and build information.

use foc_devnet::config::{Config, Location};
use foc_devnet::version_info::VersionInfo;
use std::io::IsTerminal;
use tracing::info;

/// Emit a line either as a tracing INFO event or a plain println.
macro_rules! emit {
    ($plain:expr, $fmt:literal $(, $arg:expr)*) => {
        if $plain {
            println!($fmt $(, $arg)*);
        } else {
            info!($fmt $(, $arg)*);
        }
    };
}

/// Execute the version command.
///
/// Plain output (no tracing prefixes) is used when stdout is not a terminal,
/// or when `notty` is `true` (explicit override).
pub fn handle_version(notty: bool) -> Result<(), Box<dyn std::error::Error>> {
    let plain = notty || !std::io::stdout().is_terminal();
    // Version information is read-only, no poison protection needed
    let version_info = VersionInfo::from_env();
    let dirty_suffix = if version_info.dirty.is_empty() {
        ""
    } else {
        "-dirty"
    };

    emit!(plain, "foc-devnet {}", version_info.version);
    emit!(plain, "Commit: {}{}", version_info.commit, dirty_suffix);
    emit!(plain, "Branch: {}", version_info.branch);

    let relative_time =
        format_relative_time(chrono::Utc::now().timestamp() - version_info.build_timestamp);

    emit!(
        plain,
        "Built (UTC): {} {}",
        version_info.build_time_utc,
        relative_time
    );
    emit!(plain, "Built (Local): {}", version_info.build_time_local);

    let default_config = Config::default();
    emit!(plain, "");
    print_location_info(plain, "default:code:lotus", &default_config.lotus);
    print_location_info(plain, "default:code:curio", &default_config.curio);
    print_location_info(
        plain,
        "default:code:filecoin-services",
        &default_config.filecoin_services,
    );
    print_location_info(plain, "default:code:multicall3", &default_config.multicall3);

    Ok(())
}

const SECS_PER_MIN: i64 = 60;
const SECS_PER_HOUR: i64 = 3_600;
const SECS_PER_DAY: i64 = 86_400;

/// Format a duration in seconds as a human-readable relative time string.
fn format_relative_time(diff_seconds: i64) -> String {
    if diff_seconds < SECS_PER_MIN {
        format!("({} seconds ago)", diff_seconds)
    } else if diff_seconds < SECS_PER_HOUR {
        format!("({} minutes ago)", diff_seconds / SECS_PER_MIN)
    } else if diff_seconds < SECS_PER_DAY {
        format!("({} hours ago)", diff_seconds / SECS_PER_HOUR)
    } else {
        format!("({} days ago)", diff_seconds / SECS_PER_DAY)
    }
}

/// Print location information in a formatted way.
fn print_location_info(plain: bool, label: &str, location: &Location) {
    match location {
        Location::LocalSource { dir } => {
            emit!(plain, "{}: local source at {}", label, dir);
        }
        Location::GitCommit { url, commit } => {
            emit!(plain, "{}: {}, commit {}", label, url, commit);
        }
        Location::GitTag { url, tag } => {
            emit!(plain, "{}: {}, tag {}", label, url, tag);
        }
        Location::GitBranch { url, branch } => {
            emit!(plain, "{}: {}, branch {}", label, url, branch);
        }
    }
}
