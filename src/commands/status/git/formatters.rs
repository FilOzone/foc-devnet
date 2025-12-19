//! Git information formatting utilities.
//!
//! This module provides functions for formatting git information
//! for display in status tables and reports.

use crate::commands::status::git::git_info::GitInfo;
use crate::config::Location;

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
/// use foc_localnet::commands::status::git::formatters::format_location_info;
/// use foc_localnet::commands::status::git::git_info::GitInfo;
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
        "Ready".to_string()
    } else {
        "Not Ready".to_string()
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
    use crate::config::Location;

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
