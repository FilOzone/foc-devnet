//! Code repository download utilities for foc-devnet initialization.
//!
//! This module handles the downloading and setup of Git repositories
//! for Lotus and Curio components.

use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::process::Command;
use tracing::info;

use crate::config::{Config, Location};
use crate::paths::{foc_devnet_code, foc_devnet_config};

/// Download code repositories for foc-devnet.
///
/// This function clones Git repositories for lotus and curio if their
/// locations are Git-based. It reads the repository locations from the
/// configuration file.
///
/// # Returns
/// Returns `Ok(())` if repositories are downloaded successfully, or an error if any step fails.
pub fn download_code_repositories() -> Result<(), Box<dyn std::error::Error>> {
    info!("Downloading code repositories...");

    // Load configuration
    let config_path = foc_devnet_config();
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Download lotus repository if Git-based
    download_repository("lotus", &config.lotus)?;

    // Download curio repository if Git-based
    download_repository("curio", &config.curio)?;

    // Download filecoin-services repository if Git-based
    download_repository("filecoin-services", &config.filecoin_services)?;

    // Download multicall3 repository if Git-based
    download_repository("multicall3", &config.multicall3)?;

    info!("Code repositories are now available.");
    Ok(())
}

/// Download a repository based on its location specification.
///
/// This function handles different types of repository locations:
/// - LocalSource: Skips download
/// - GitCommit: Clones and checks out specific commit
/// - GitTag: Clones and checks out specific tag
/// - GitBranch: Clones and checks out specific branch
///
/// # Arguments
/// * `name` - Name of the repository (e.g., "lotus", "curio")
/// * `location` - Location specification for the repository
///
/// # Returns
/// Returns `Ok(())` if repository is downloaded successfully, or an error if download fails.
fn download_repository(name: &str, location: &Location) -> Result<(), Box<dyn std::error::Error>> {
    match location {
        Location::LocalSource { dir } => {
            info!(
                "{} using local source at {}, creating symlink...",
                name, dir
            );
            let target_link = foc_devnet_code().join(name);

            if target_link.exists() {
                // If it exists, we check if it's a symlink or a directory
                if fs::symlink_metadata(&target_link)?.file_type().is_symlink() {
                    // It's a symlink, remove it and recreate it to ensure it points to the new location
                    fs::remove_file(&target_link)?;
                } else {
                    // It's a directory (likely from a previous git clone)
                    // We should probably back it up or warn, but for now let's just warn and return
                    // or maybe we should remove it?
                    // Let's be safe and just warn if it's not a symlink
                    info!("{} repository already exists at {} (not a symlink). Skipping symlink creation.", name, target_link.display());
                    return Ok(());
                }
            }

            // Create symlink
            #[cfg(unix)]
            std::os::unix::fs::symlink(dir, &target_link)?;
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(dir, &target_link)?;

            info!("Created symlink for {} at {}", name, target_link.display());
            Ok(())
        }
        Location::GitCommit { url, commit } => {
            clone_and_checkout(name, url, Some(commit), None, None)
        }
        Location::GitTag { url, tag } => clone_and_checkout(name, url, None, Some(tag), None),
        Location::GitBranch { url, branch } => {
            clone_and_checkout(name, url, None, None, Some(branch))
        }
    }
}

/// Clone a Git repository and checkout to the specified ref.
///
/// This function performs the complete repository setup:
/// 1. Clones the repository
/// 2. Checks out to the specified reference
/// 3. Updates submodules
///
/// # Arguments
/// * `name` - Name of the repository
/// * `url` - Git repository URL
/// * `commit` - Optional commit hash to checkout
/// * `tag` - Optional tag to checkout
/// * `branch` - Optional branch to checkout
///
/// # Returns
/// Returns `Ok(())` if repository is cloned and checked out successfully.
fn clone_and_checkout(
    name: &str,
    url: &str,
    commit: Option<&str>,
    tag: Option<&str>,
    branch: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let repo_dir = foc_devnet_code().join(name);

    if repo_dir.exists() {
        info!(
            "{} repository already exists at {}",
            name,
            repo_dir.display()
        );
        return Ok(());
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Cloning {}...", name));

    let status = Command::new("git")
        .args(["clone", url, repo_dir.to_str().unwrap()])
        .status()?;

    if !status.success() {
        pb.finish_with_message(format!("Failed to clone {}", name));
        return Err(format!("Failed to clone repository from {}", url).into());
    }

    // Checkout to specific ref if provided
    if let Some(c) = commit {
        pb.set_message(format!("Checking out commit {} for {}...", c, name));
        let status = Command::new("git")
            .args(["checkout", c])
            .current_dir(&repo_dir)
            .status()?;
        if !status.success() {
            pb.finish_with_message(format!("Failed to checkout commit {} for {}", c, name));
            return Err(format!("Failed to checkout commit {}", c).into());
        }
    } else if let Some(t) = tag {
        pb.set_message(format!("Checking out tag {} for {}...", t, name));
        let status = Command::new("git")
            .args(["checkout", &format!("tags/{}", t)])
            .current_dir(&repo_dir)
            .status()?;
        if !status.success() {
            pb.finish_with_message(format!("Failed to checkout tag {} for {}", t, name));
            return Err(format!("Failed to checkout tag {}", t).into());
        }
    } else if let Some(b) = branch {
        pb.set_message(format!("Checking out branch {} for {}...", b, name));
        let status = Command::new("git")
            .args(["checkout", b])
            .current_dir(&repo_dir)
            .status()?;
        if !status.success() {
            pb.finish_with_message(format!("Failed to checkout branch {} for {}", b, name));
            return Err(format!("Failed to checkout branch {}", b).into());
        }
    }

    // Update submodules
    pb.set_message(format!("Updating submodules for {}...", name));
    let status = Command::new("git")
        .args(["submodule", "update", "--init", "--recursive"])
        .current_dir(&repo_dir)
        .status()?;

    if !status.success() {
        pb.finish_with_message(format!("Failed to update submodules for {}", name));
        return Err(format!("Failed to update submodules for {}", name).into());
    }

    pb.finish_with_message(format!("{} repository ready", name));
    Ok(())
}
