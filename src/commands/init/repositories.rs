//! Code repository download utilities for foc-localnet initialization.
//!
//! This module handles the downloading and setup of Git repositories
//! for Lotus and Curio components.

use crossterm::style::Stylize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;
use std::process::Command;

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
    println!("{}", "Downloading code repositories...".bold());

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

    println!("  {} Code repositories are now available.", "✓".green());
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
            println!(
                "  {} {} using local source, skipping download",
                "✓".green(),
                name
            );
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
        println!(
            "  {} {} repository already exists at {}",
            "✓".green(),
            name,
            repo_dir.display()
        );
        return Ok(());
    }

    clone_repo(name, url, &repo_dir)?;
    let checkout_ref = determine_checkout_ref(commit, tag, branch);
    checkout_to_ref(name, &repo_dir, &checkout_ref)?;
    update_submodules(name, &repo_dir)?;

    println!(
        "{} Cloned and checked out {} to {}",
        "✓".green(),
        name,
        checkout_ref
    );
    Ok(())
}

/// Clone the repository from the given URL to the specified directory.
///
/// # Arguments
/// * `name` - Name of the repository for progress messages
/// * `url` - Git repository URL
/// * `repo_dir` - Directory to clone into
///
/// # Returns
/// Returns `Ok(())` if cloning succeeds, or an error if cloning fails.
fn clone_repo(name: &str, url: &str, repo_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("  {} Cloning {} from {}...", "📥".bold(), name, url);

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Cloning {} repository...", name));

    // Clone the repository
    let status = Command::new("git")
        .args(["clone", url, &repo_dir.to_string_lossy()])
        .status()?;

    if !status.success() {
        pb.finish_with_message(format!("❌ Failed to clone {} repository", name));
        return Err(format!("Failed to clone {} repository", name).into());
    }

    pb.finish();
    Ok(())
}

/// Determine the reference to checkout based on the provided options.
///
/// Priority order: commit > tag > branch > "main"
///
/// # Arguments
/// * `commit` - Optional commit hash
/// * `tag` - Optional tag name
/// * `branch` - Optional branch name
///
/// # Returns
/// Returns the reference string to checkout.
fn determine_checkout_ref(commit: Option<&str>, tag: Option<&str>, branch: Option<&str>) -> String {
    if let Some(commit) = commit {
        commit.to_string()
    } else if let Some(tag) = tag {
        tag.to_string()
    } else if let Some(branch) = branch {
        branch.to_string()
    } else {
        "main".to_string()
    }
}

/// Checkout to the specified reference in the repository.
///
/// # Arguments
/// * `name` - Name of the repository for progress messages
/// * `repo_dir` - Repository directory
/// * `checkout_ref` - Reference to checkout (commit, tag, or branch)
///
/// # Returns
/// Returns `Ok(())` if checkout succeeds, or an error if checkout fails.
fn checkout_to_ref(
    name: &str,
    repo_dir: &Path,
    checkout_ref: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Checking out {}...", name));

    let status = Command::new("git")
        .args(["checkout", checkout_ref])
        .current_dir(repo_dir)
        .status()?;

    if !status.success() {
        pb.finish_with_message(format!(
            "❌ Failed to checkout {} to {}",
            name, checkout_ref
        ));
        return Err(format!("Failed to checkout {} to {}", name, checkout_ref).into());
    }

    pb.finish();
    Ok(())
}

/// Update submodules in the repository.
///
/// # Arguments
/// * `name` - Name of the repository for progress messages
/// * `repo_dir` - Repository directory
///
/// # Returns
/// Returns `Ok(())` if submodule update succeeds, or an error if it fails.
fn update_submodules(name: &str, repo_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Updating submodules for {}...", name));

    // Update submodules recursively
    let status = Command::new("git")
        .args(["submodule", "update", "--init", "--recursive"])
        .current_dir(repo_dir)
        .status()?;

    if !status.success() {
        pb.finish_with_message(format!("❌ Failed to update submodules for {}", name));
        return Err(format!("Failed to update submodules for {}", name).into());
    }

    pb.finish();
    Ok(())
}
