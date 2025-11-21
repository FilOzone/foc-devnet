//! # Git Utilities
//!
//! This module provides utilities for interacting with Git repositories,
//! including version detection, branch information, and commit hashes.
//!
//! It supports various source types defined in the configuration:
//! - Local source directories
//! - Git tags
//! - Git commits
//! - Git branches

use crate::config::Location;
use crate::paths::foc_localnet_code;
use std::process::Command;

/// Enum representing different types of git version information.
///
/// This enum encapsulates the various ways we can identify a git repository state.
#[derive(Debug, Clone, PartialEq)]
pub enum GitInfo {
    /// A specific git tag is checked out
    Tag(String),
    /// A branch is checked out with its current commit hash
    Branch(String, String), // branch name, commit hash
    /// A specific commit hash is checked out (detached HEAD)
    Commit(String),
    /// No git information available
    None,
}

/// Get git version information for a specific repository.
///
/// This function attempts to determine the current git state of a repository
/// by checking for tags, branches, and commit hashes in order of preference.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::git::get_git_info;
/// use std::path::Path;
///
/// let repo_path = Path::new("/path/to/repo");
/// let info = get_git_info(repo_path).unwrap();
/// match info {
///     GitInfo::Tag(tag) => println!("On tag: {}", tag),
///     GitInfo::Branch(branch, commit) => println!("On branch {} at {}", branch, commit),
///     GitInfo::Commit(commit) => println!("At commit {}", commit),
///     GitInfo::None => println!("No git info available"),
/// }
/// ```
///
/// # Errors
///
/// Returns an error if git commands fail to execute or if the repository path is invalid.
pub fn get_git_info(repo_path: &std::path::Path) -> Result<GitInfo, Box<dyn std::error::Error>> {
    // Try to get tag first
    if let Ok(tag_output) = Command::new("git")
        .args([
            "-C",
            repo_path.to_str().unwrap_or("."),
            "describe",
            "--tags",
            "--exact-match",
        ])
        .output()
    {
        if tag_output.status.success() {
            let tag = String::from_utf8_lossy(&tag_output.stdout)
                .trim()
                .to_string();
            return Ok(GitInfo::Tag(tag));
        }
    }

    // Try to get branch and commit
    if let Ok(branch_output) = Command::new("git")
        .args([
            "-C",
            repo_path.to_str().unwrap_or("."),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ])
        .output()
    {
        if branch_output.status.success() {
            let branch = String::from_utf8_lossy(&branch_output.stdout)
                .trim()
                .to_string();

            if let Ok(commit_output) = Command::new("git")
                .args(["-C", repo_path.to_str().unwrap_or("."), "rev-parse", "HEAD"])
                .output()
            {
                if commit_output.status.success() {
                    let commit = String::from_utf8_lossy(&commit_output.stdout)
                        .trim()
                        .to_string();
                    return Ok(GitInfo::Branch(branch, commit));
                }
            }
        }
    }

    // Fallback to just commit hash
    if let Ok(commit_output) = Command::new("git")
        .args(["-C", repo_path.to_str().unwrap_or("."), "rev-parse", "HEAD"])
        .output()
    {
        if commit_output.status.success() {
            let commit = String::from_utf8_lossy(&commit_output.stdout)
                .trim()
                .to_string();
            return Ok(GitInfo::Commit(commit));
        }
    }

    Ok(GitInfo::None)
}

/// Get the repository path to check for git information based on the config location.
///
/// This function determines which directory to check for git information
/// based on the location configuration.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::git::get_repo_path_from_config;
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

/// Format location and git information for display.
///
/// This function combines location configuration and git information to produce
/// formatted strings suitable for display in status tables.
///
/// # Returns
///
/// A tuple containing:
/// - Source type description
/// - Version/branch/tag information
/// - Commit hash (shortened)
/// - Status indicator ("Ready" or "Not Ready")
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::git::{format_location_info, GitInfo};
/// use foc_localnet::config::{Location, GitBranch};
/// use std::path::Path;
///
/// let location = Location::GitBranch(GitBranch {
///     url: "https://github.com/example/repo".to_string(),
///     branch: "main".to_string(),
/// });
/// let git_info = GitInfo::Branch("main".to_string(), "abc123def456".to_string());
/// let repo_path = Path::new("/path/to/repo");
///
/// let (source_type, version, commit, status) = format_location_info(&location, &git_info, repo_path);
/// ```
pub fn format_location_info(
    location: &Location,
    git_info: &GitInfo,
    _repo_path: &std::path::Path,
) -> (String, String, String, String) {
    use crossterm::style::Stylize;

    let is_ready = match (location, git_info) {
        // LocalSource is ready if it has any git info
        (
            Location::LocalSource { .. },
            GitInfo::Tag(_) | GitInfo::Branch(_, _) | GitInfo::Commit(_),
        ) => true,
        (Location::LocalSource { .. }, GitInfo::None) => false,

        // GitTag is ready if the repository has that exact tag checked out
        (
            Location::GitTag {
                tag: expected_tag, ..
            },
            GitInfo::Tag(actual_tag),
        ) if expected_tag == actual_tag => true,
        (Location::GitTag { .. }, _) => false,

        // GitCommit is ready if the repository is at that exact commit
        (
            Location::GitCommit {
                commit: expected_commit,
                ..
            },
            GitInfo::Commit(actual_commit),
        ) if expected_commit == actual_commit => true,
        (Location::GitCommit { .. }, _) => false,

        // GitBranch is ready if the repository is on that branch (or has that branch's commit/tag)
        (
            Location::GitBranch {
                branch: expected_branch,
                ..
            },
            GitInfo::Branch(actual_branch, _),
        ) if expected_branch == actual_branch => true,
        (Location::GitBranch { .. }, GitInfo::Tag(_) | GitInfo::Commit(_)) => true, // Assume it's ready if we have some valid state
        (Location::GitBranch { .. }, _) => false,
    };

    let status = if is_ready {
        "Ready".green().to_string()
    } else {
        "Not Ready".red().to_string()
    };

    let (source_type, version, commit) = match location {
        Location::LocalSource { dir: _ } => match git_info {
            GitInfo::Tag(tag) => ("Local (Git Tag)".to_string(), tag.clone(), "".to_string()),
            GitInfo::Branch(branch, commit) => (
                "Local (Git Branch)".to_string(),
                branch.clone(),
                format!("{}...", &commit[..8]),
            ),
            GitInfo::Commit(commit) => (
                "Local (Git Commit)".to_string(),
                format!("{}...", &commit[..8]),
                "".to_string(),
            ),
            GitInfo::None => ("Local".to_string(), "Not found".to_string(), "".to_string()),
        },
        Location::GitTag { tag, .. } => ("Git Tag".to_string(), tag.clone(), "".to_string()),
        Location::GitCommit { commit, .. } => (
            "Git Commit".to_string(),
            format!("{}...", &commit[..8]),
            "".to_string(),
        ),
        Location::GitBranch { branch, .. } => match git_info {
            GitInfo::Branch(git_branch, commit) => (
                "Git Branch".to_string(),
                git_branch.clone(),
                format!("{}...", &commit[..8]),
            ),
            GitInfo::Tag(tag) => ("Git Branch + Tag".to_string(), tag.clone(), "".to_string()),
            GitInfo::Commit(commit) => (
                "Git Branch + Commit".to_string(),
                format!("{}...", &commit[..8]),
                "".to_string(),
            ),
            GitInfo::None => (
                "Git Branch".to_string(),
                branch.clone(),
                "Not found".to_string(),
            ),
        },
    };

    (source_type, version, commit, status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_location_info_local_source() {
        let location = Location::LocalSource {
            dir: "/some/path".to_string(),
        };
        let git_info = GitInfo::Branch("main".to_string(), "abc123def456".to_string());
        let repo_path = std::path::Path::new("/some/path");

        let (source_type, version, commit, status) =
            format_location_info(&location, &git_info, repo_path);

        assert_eq!(source_type, "Local (Git Branch)");
        assert_eq!(version, "main");
        assert_eq!(commit, "abc123de...");
        assert!(status.contains("Ready"));
    }

    #[test]
    fn test_format_location_info_git_tag() {
        let location = Location::GitTag {
            url: "https://github.com/example/repo".to_string(),
            tag: "v1.0.0".to_string(),
        };
        let git_info = GitInfo::Tag("v1.0.0".to_string());
        let repo_path = std::path::Path::new("/some/path");

        let (source_type, version, commit, status) =
            format_location_info(&location, &git_info, repo_path);

        assert_eq!(source_type, "Git Tag");
        assert_eq!(version, "v1.0.0");
        assert_eq!(commit, "");
        assert!(status.contains("Ready"));
    }
}
