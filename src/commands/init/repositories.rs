//! Code repository download utilities for foc-localnet initialization.
//!
//! This module handles the downloading and setup of Git repositories
//! for Lotus and Curio components.

use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::process::Command;
use tracing::info;

use crate::config::{Config, Location};
use crate::paths::{foc_localnet_code, foc_localnet_config};

/// Download code repositories for foc-localnet.
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
    let config_path = foc_localnet_config();
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

    // Download synapse-sdk repository if Git-based
    download_repository("synapse-sdk", &config.synapse_sdk)?;

    info!("  Code repositories are now available.");
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
        Location::LocalSource { .. } => {
            info!("  {} using local source, skipping download", name);
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
    let repo_dir = foc_localnet_code().join(name);

    if repo_dir.exists() {
        info!(
            "  {} repository already exists at {}",
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
