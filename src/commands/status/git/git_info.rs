//! Git information utilities.
//!
//! This module provides the GitInfo enum and functions for retrieving
//! git repository information such as tags, branches, and commits.

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
