// Git utilities for repository status and information.
//
// This module provides utilities for interacting with Git repositories,
// including version detection, branch information, and commit hashes.
//
// It supports various source types defined in the configuration:
// - Local source directories
// - Git tags
// - Git commits
// - Git branches

pub mod formatters;
pub mod git_info;
pub mod repo_paths;

// Re-export the main types and functions for convenience
pub use formatters::format_location_info;
pub use git_info::{get_git_info, GitInfo};
pub use repo_paths::get_repo_path_from_config;