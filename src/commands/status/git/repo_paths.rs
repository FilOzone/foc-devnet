//! Repository path utilities.
//!
//! This module provides utilities for determining repository paths
//! based on configuration locations.

use crate::config::Location;
use crate::paths::foc_devnet_code;

/// Get the repository path to check for git information based on the config location.
///
/// This function determines which directory to check for git information
/// based on the location configuration.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_devnet::commands::status::git::repo_paths::get_repo_path_from_config;
/// use foc_devnet::config::{Location, GitBranch};
///
/// let location = Location::GitBranch(GitBranch {
///     url: "https://github.com/example/repo".to_string(),
///     branch: "main".to_string(),
/// });
/// let path = get_repo_path_from_config(&location, "component");
/// ```
///
/// # Parameters
///
/// * `location` - The location configuration for the component
/// * `component` - The component name (e.g., "lotus", "curio")
pub fn get_repo_path_from_config(location: &Location, component: &str) -> std::path::PathBuf {
    match location {
        Location::LocalSource { dir } => {
            // For local sources, check the specified directory
            std::path::PathBuf::from(dir)
        }
        Location::GitTag { .. }
        | Location::GitCommit { .. }
        | Location::GitBranch { .. }
        | Location::LatestCommit { .. }
        | Location::LatestTag { .. } => {
            // For git sources (including unresolved dynamic variants), use the foc-devnet code directory
            foc_devnet_code().join(component)
        }
    }
}
