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
use crossterm::style::Stylize;
use std::fs;
use tabular::{Row, Table};

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
    println!("\n{} {}", "📋".cyan(), "Code Versions".bold().cyan());
    println!("{}", "─".repeat(120).cyan());

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

    // Create tabular output with proper column widths
    let mut table = Table::new("{:<}  {:<}  {:<}  {:<}  {:<}  {:<}");
    table.add_row(
        Row::new()
            .with_ansi_cell("Repository".bold().dark_grey())
            .with_ansi_cell("Source Type".bold().dark_grey())
            .with_ansi_cell("Branch/Tag".bold().dark_grey())
            .with_ansi_cell("Commit".bold().dark_grey())
            .with_ansi_cell("Status".bold().dark_grey())
            .with_ansi_cell("Path".bold().dark_grey()),
    );

    // Use with_ansi_cell for colored output
    table.add_row(
        Row::new()
            .with_ansi_cell("Lotus".cyan())
            .with_ansi_cell(&lotus_source_type)
            .with_ansi_cell(&lotus_version)
            .with_ansi_cell(&lotus_commit)
            .with_ansi_cell(&lotus_status)
            .with_ansi_cell(lotus_repo_path.display().to_string().dim()),
    );

    table.add_row(
        Row::new()
            .with_ansi_cell("Curio".magenta())
            .with_ansi_cell(&curio_source_type)
            .with_ansi_cell(&curio_version)
            .with_ansi_cell(&curio_commit)
            .with_ansi_cell(&curio_status)
            .with_ansi_cell(curio_repo_path.display().to_string().dim()),
    );

    print!("{}", table);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Note: These tests would require setting up mock git repositories
    // and configuration files, which is complex. In a real implementation,
    // you might want to use mocking libraries or integration tests.

    #[test]
    fn test_print_code_version_requires_config() {
        // This test will fail if no config file exists, which is expected
        // in a test environment. We just verify the function signature works.
        let result = print_code_version();
        // We expect this to fail in test environment due to missing config
        assert!(result.is_err());
    }
}
