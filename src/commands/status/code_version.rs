//! # Code Version Status
//!
//! This module handles the display of code version information for the foc-localnet system.
//!
//! It provides functionality to:
//! - Display git repository information for Lotus and Curio
//! - Show source types (local, git tag, git branch, git commit)
//! - Indicate readiness status of code repositories

use crate::config::Config;
use crate::paths::foc_localnet_config;
use std::fs;
use tabular::{Row, Table};
use tracing::info;

use super::git::{format_location_info, get_git_info, get_repo_path_from_config};

/// Print code version information in tabular format.
///
/// This function displays version information for both Lotus and Curio repositories,
/// including their source types, current versions, commit hashes, and readiness status.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::code_version::print_code_version;
///
/// print_code_version().expect("Failed to print code version");
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The configuration file cannot be read or parsed
/// - Git repository information cannot be retrieved
pub fn print_code_version() -> Result<(), Box<dyn std::error::Error>> {
    info!("Code Versions");

    // Load configuration
    let config_path = foc_localnet_config();
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Get git information for Lotus
    let lotus_repo_path = get_repo_path_from_config(&config.lotus, "lotus");
    let lotus_git_info = get_git_info(&lotus_repo_path)?;

    let (lotus_source_type, lotus_version, lotus_commit, lotus_status) =
        format_location_info(&config.lotus, &lotus_git_info, &lotus_repo_path);

    // Get git information for Curio
    let curio_repo_path = get_repo_path_from_config(&config.curio, "curio");
    let curio_git_info = get_git_info(&curio_repo_path)?;

    let (curio_source_type, curio_version, curio_commit, curio_status) =
        format_location_info(&config.curio, &curio_git_info, &curio_repo_path);

    // Create table
    let mut table = Table::new("{:<15} {:<15} {:<15} {:<15} {:<15}");
    table.add_row(
        Row::new()
            .with_cell("Component")
            .with_cell("Source Type")
            .with_cell("Version")
            .with_cell("Commit")
            .with_cell("Status"),
    );

    table.add_row(
        Row::new()
            .with_cell("Lotus")
            .with_cell(lotus_source_type)
            .with_cell(lotus_version)
            .with_cell(lotus_commit)
            .with_cell(lotus_status),
    );

    table.add_row(
        Row::new()
            .with_cell("Curio")
            .with_cell(curio_source_type)
            .with_cell(curio_version)
            .with_cell(curio_commit)
            .with_cell(curio_status),
    );

    for line in table.to_string().lines() {
        info!("{}", line);
    }

    Ok(())
}
