//! Version information display.
//!
//! This module handles displaying version and build information.

use foc_devnet::config::{Config, Location};
use foc_devnet::version_info::VersionInfo;
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
/// When `noterminal` is true, output is printed without tracing prefixes.
pub fn handle_version(noterminal: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Version information is read-only, no poison protection needed
    let version_info = VersionInfo::from_env();
    let dirty_suffix = if version_info.dirty.is_empty() {
        ""
    } else {
        "-dirty"
    };

    emit!(noterminal, "foc-devnet {}", version_info.version);
    emit!(
        noterminal,
        "Commit: {}{}",
        version_info.commit,
        dirty_suffix
    );
    emit!(noterminal, "Branch: {}", version_info.branch);

    let relative_time =
        format_relative_time(chrono::Utc::now().timestamp() - version_info.build_timestamp);

    emit!(
        noterminal,
        "Built (UTC): {} {}",
        version_info.build_time_utc,
        relative_time
    );
    emit!(
        noterminal,
        "Built (Local): {}",
        version_info.build_time_local
    );

    let default_config = Config::default();
    emit!(noterminal, "");
    print_location_info(noterminal, "default:code:lotus", &default_config.lotus);
    print_location_info(noterminal, "default:code:curio", &default_config.curio);
    print_location_info(
        noterminal,
        "default:code:filecoin-services",
        &default_config.filecoin_services,
    );
    print_location_info(
        noterminal,
        "default:code:multicall3",
        &default_config.multicall3,
    );
    emit!(
        noterminal,
        "default:yugabyte: {}",
        default_config.yugabyte_download_url
    );

    Ok(())
}

/// Format a duration in seconds as a human-readable relative time string.
fn format_relative_time(diff_seconds: i64) -> String {
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
