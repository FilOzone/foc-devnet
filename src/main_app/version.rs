//! Version information display.
//!
//! This module handles displaying version and build information.

use foc_devnet::config::{Config, Location};
use foc_devnet::version_info::VersionInfo;
use tracing::info;

/// Execute the version command
pub fn handle_version() -> Result<(), Box<dyn std::error::Error>> {
    // Version information is read-only, no poison protection needed
    let version_info = VersionInfo::from_env();
    let dirty_suffix = if version_info.dirty.is_empty() {
        ""
    } else {
        "-dirty"
    };

    info!("foc-devnet {}", version_info.version);
    info!("Commit: {}{}", version_info.commit, dirty_suffix);
    info!("Branch: {}", version_info.branch);

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

    info!(
        "Built (UTC): {} {}",
        version_info.build_time_utc, relative_time
    );
    info!("Built (Local): {}", version_info.build_time_local);

    // Print default configuration values
    let default_config = Config::default();
    info!("");
    print_location_info("default:code:lotus", &default_config.lotus);
    print_location_info("default:code:curio", &default_config.curio);
    print_location_info(
        "default:code:filecoin-services",
        &default_config.filecoin_services,
    );
    print_location_info("default:code:multicall3", &default_config.multicall3);
    info!("default:yugabyte: {}", default_config.yugabyte_download_url);

    Ok(())
}

/// Print location information in a formatted way
fn print_location_info(label: &str, location: &Location) {
    match location {
        Location::LocalSource { dir } => {
            info!("{}: local source at {}", label, dir);
        }
        Location::GitCommit { url, commit } => {
            info!("{}: {}, commit {}", label, url, commit);
        }
        Location::GitTag { url, tag } => {
            info!("{}: {}, tag {}", label, url, tag);
        }
        Location::GitBranch { url, branch } => {
            info!("{}: {}, branch {}", label, url, branch);
        }
        Location::LatestCommit { url } => {
            info!("{}: {}, latest commit (unresolved)", label, url);
        }
        Location::LatestTag { url } => {
            info!("{}: {}, latest tag (unresolved)", label, url);
        }
    }
}
