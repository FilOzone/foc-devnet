//! Repository preparation module.
//!
//! This module handles preparing source code repositories before building.
//! It supports cloning from Git (with commit/tag/branch checkout) and symlinking local directories.

use crate::commands::build::Project;
use crate::config::Location;
use crate::paths::{foc_devnet_curio_repo, foc_devnet_lotus_repo};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

/// Prepare a repository for building based on the Location configuration.
///
/// This function handles:
/// - Cloning Git repositories and checking out specific commits/tags/branches
/// - Creating symlinks for local source directories
/// - Cleaning up previous setups when switching between different Location types
///
/// Returns the path to the prepared repository.
pub fn prepare_repository(
    project: &Project,
    location: &Location,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repo_path = match project {
        Project::Lotus => foc_devnet_lotus_repo(),
        Project::Curio => foc_devnet_curio_repo(),
    };

    info!(
        "Preparing {} repository at {}...",
        project,
        repo_path.display()
    );

    match location {
        Location::LocalSource { dir } => {
            prepare_local_source(&repo_path, dir)?;
        }
        Location::GitCommit { url, commit } => {
            prepare_git_repo(&repo_path, url)?;
            checkout_commit(&repo_path, commit)?;
        }
        Location::GitTag { url, tag } => {
            prepare_git_repo(&repo_path, url)?;
            checkout_tag(&repo_path, tag)?;
        }
        Location::GitBranch { url, branch } => {
            prepare_git_repo(&repo_path, url)?;
            checkout_branch(&repo_path, branch)?;
        }
        Location::LatestTag { .. } => {
            return Err(format!(
                "{}: LatestTag should have been resolved at init time",
                project
            )
            .into());
        }
    }

    info!("Repository prepared successfully");
    Ok(repo_path)
}

/// Prepare a local source directory by creating a symlink.
///
/// If the target path already exists, it will be removed first.
fn prepare_local_source(
    repo_path: &PathBuf,
    source_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_path = Path::new(source_dir);

    if !source_path.exists() {
        return Err(format!("Local source directory does not exist: {}", source_dir).into());
    }

    // Clean up existing path
    cleanup_repo_path(repo_path)?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = repo_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create symlink
    info!(
        "Creating symlink from {} to {}",
        repo_path.display(),
        source_dir
    );
    #[cfg(unix)]
    std::os::unix::fs::symlink(source_path, repo_path)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(source_path, repo_path)?;

    Ok(())
}

/// Prepare a Git repository by cloning or updating it.
///
/// If the repository already exists and is a valid Git repo, it will be updated (fetch).
/// Otherwise, it will be cloned fresh.
fn prepare_git_repo(repo_path: &PathBuf, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let git_dir = repo_path.join(".git");

    if repo_path.exists() {
        handle_existing_repo_path(repo_path, &git_dir, url)?;
    } else {
        clone_fresh_repo(repo_path, url)?;
    }

    Ok(())
}

/// Handle the case where the repository path already exists.
fn handle_existing_repo_path(
    repo_path: &PathBuf,
    git_dir: &Path,
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if it's a symlink - if so, remove it
    if repo_path.is_symlink() {
        info!(
            "Removing symlink at {} to replace with Git repository",
            repo_path.display()
        );
        fs::remove_file(repo_path)?;
        clone_fresh_repo(repo_path, url)?;
    } else if git_dir.exists() {
        // It's already a Git repository, update it
        update_existing_repo(repo_path)?;
    } else {
        // Path exists but is not a Git repo or symlink, remove it
        info!(
            "Removing existing non-Git directory at {}",
            repo_path.display()
        );
        fs::remove_dir_all(repo_path)?;
        clone_fresh_repo(repo_path, url)?;
    }
    Ok(())
}

/// Update an existing Git repository by fetching latest changes.
fn update_existing_repo(repo_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Updating existing Git repository at {}",
        repo_path.display()
    );

    let status = Command::new("git")
        .args(["fetch", "--all", "--tags", "--prune"])
        .current_dir(repo_path)
        .status()?;

    if !status.success() {
        return Err(format!(
            "Failed to fetch updates for repository at {}",
            repo_path.display()
        )
        .into());
    }

    Ok(())
}

/// Clone a fresh Git repository from the given URL.
fn clone_fresh_repo(repo_path: &Path, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create parent directory
    if let Some(parent) = repo_path.parent() {
        fs::create_dir_all(parent)?;
    }

    info!("Cloning repository from {} to {}", url, repo_path.display());

    let status = Command::new("git")
        .args(["clone", url, repo_path.to_str().unwrap()])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to clone repository from {}", url).into());
    }

    Ok(())
}

/// Clean up the repository path, handling both symlinks and directories.
fn cleanup_repo_path(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_symlink() {
        info!("Removing existing symlink at {}", path.display());
        fs::remove_file(path)?;
    } else if path.is_dir() {
        info!("Removing existing directory at {}", path.display());
        fs::remove_dir_all(path)?;
    } else {
        info!("Removing existing file at {}", path.display());
        fs::remove_file(path)?;
    }

    Ok(())
}

/// Checkout a specific commit in a Git repository.
fn checkout_commit(repo_path: &PathBuf, commit: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Checking out commit: {}", commit);

    let status = Command::new("git")
        .args(["checkout", commit])
        .current_dir(repo_path)
        .status()?;

    if !status.success() {
        return Err(format!("Failed to checkout commit {}", commit).into());
    }

    Ok(())
}

/// Checkout a specific tag in a Git repository.
fn checkout_tag(repo_path: &PathBuf, tag: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Checking out tag: {}", tag);

    let status = Command::new("git")
        .args(["checkout", &format!("tags/{}", tag)])
        .current_dir(repo_path)
        .status()?;

    if !status.success() {
        return Err(format!("Failed to checkout tag {}", tag).into());
    }

    Ok(())
}

/// Checkout a specific branch in a Git repository.
fn checkout_branch(repo_path: &PathBuf, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Checking out branch: {}", branch);

    // First try to checkout the branch if it exists locally
    let status = Command::new("git")
        .args(["checkout", branch])
        .current_dir(repo_path)
        .status()?;

    if !status.success() {
        // If that fails, try to checkout from remote
        let status = Command::new("git")
            .args(["checkout", "-b", branch, &format!("origin/{}", branch)])
            .current_dir(repo_path)
            .status()?;

        if !status.success() {
            return Err(format!("Failed to checkout branch {}", branch).into());
        }
    }

    // Pull latest changes for the branch
    info!("Pulling latest changes for branch: {}", branch);
    let status = Command::new("git")
        .args(["pull", "origin", branch])
        .current_dir(repo_path)
        .status()?;

    if !status.success() {
        return Err(format!("Failed to pull latest changes for branch {}", branch).into());
    }

    Ok(())
}
