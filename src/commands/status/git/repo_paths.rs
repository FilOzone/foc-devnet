//! Repository path utilities.
//!
//! This module provides utilities for determining repository paths
//! based on configuration locations.

use crate::config::Location;
use crate::paths::foc_localnet_code;

/// Get the repository path to check for git information based on the config location.
///
/// This function determines which directory to check for git information
/// based on the location configuration.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::git::repo_paths::get_repo_path_from_config;
/// use foc_localnet::config::{Location, GitBranch};
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
        Location::GitTag { .. } | Location::GitCommit { .. } | Location::GitBranch { .. } => {
            // For git sources, check if it exists in the foc-localnet code directory
            foc_localnet_code().join(component)
        }
    }
}