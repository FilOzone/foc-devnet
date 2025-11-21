//! # Disk Usage
//!
//! This module handles the display of disk usage information for foc-localnet directories.
//!
//! It provides functionality to:
//! - Calculate directory sizes
//! - Display formatted size information
//! - Show breakdown by directory type

use crate::paths::{
    foc_localnet_artifacts, foc_localnet_bin, foc_localnet_code, foc_localnet_docker_images,
    foc_localnet_docker_volumes, foc_localnet_home, foc_localnet_logs, foc_localnet_state,
    foc_localnet_tmp,
};
use crossterm::style::Stylize;
use tabular::{Row, Table};

use super::utils::{format_size, get_directory_size, get_terminal_width};

/// Print disk usage information for foc-localnet directories.
///
/// This function displays disk usage statistics for all foc-localnet directories,
/// including code, binaries, logs, state, temporary files, and artifacts.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_localnet::commands::status::disk_usage::print_disk_usage;
///
/// print_disk_usage().expect("Failed to print disk usage");
/// ```
///
/// # Errors
///
/// Returns an error if directory size calculations fail.
pub fn print_disk_usage() -> Result<(), Box<dyn std::error::Error>> {
    let width = get_terminal_width().min(120);
    let header_text = format!("{} {}", "💾".blue(), "Disk Usage");
    // Display width: 💾 (2) + space (1) + "Disk Usage" (10) + space (1) = 14
    let header_display_width = 2 + 1 + 10 + 1;
    let padding_len = width.saturating_sub(header_display_width);
    let padding = "░".repeat(padding_len).dark_grey();
    println!("\n{}{}{}", header_text.bold().blue(), " ", padding);
    let width = get_terminal_width().min(120);
    println!("{}", "─".repeat(width).blue());

    let home_dir = foc_localnet_home();

    // Get disk usage for main directories
    let mut table = Table::new("{:<}  {:<}  {:<}");
    table.add_row(
        Row::new()
            .with_ansi_cell("Directory".bold().dark_grey())
            .with_ansi_cell("Size".bold().dark_grey())
            .with_ansi_cell("Path".bold().dark_grey()),
    );

    // Main directories
    let directories = vec![
        ("Code", foc_localnet_code()),
        ("Binaries", foc_localnet_bin()),
        ("Logs", foc_localnet_logs()),
        ("State", foc_localnet_state()),
        ("Temp", foc_localnet_tmp()),
    ];

    for (name, path) in directories {
        let size = get_directory_size(&path)?;
        table.add_row(
            Row::new()
                .with_cell(name)
                .with_ansi_cell(format_size(size))
                .with_ansi_cell(path.display().to_string().dim()),
        );
    }

    // Artifacts breakdown
    let artifacts_dir = foc_localnet_artifacts();
    let artifacts_size = get_directory_size(&artifacts_dir)?;

    table.add_row(
        Row::new()
            .with_ansi_cell("Artifacts (Overall)".bold())
            .with_ansi_cell(format_size(artifacts_size).bold())
            .with_ansi_cell(artifacts_dir.display().to_string().dim()),
    );

    // Docker images
    let docker_images_dir = foc_localnet_docker_images();
    let docker_images_size = get_directory_size(&docker_images_dir)?;
    table.add_row(
        Row::new()
            .with_cell("  └─ Docker Images")
            .with_ansi_cell(format_size(docker_images_size))
            .with_ansi_cell(docker_images_dir.display().to_string().dim()),
    );

    // Docker volumes
    let docker_volumes_dir = foc_localnet_docker_volumes();
    let docker_volumes_size = get_directory_size(&docker_volumes_dir)?;
    table.add_row(
        Row::new()
            .with_cell("  └─ Docker Volumes")
            .with_ansi_cell(format_size(docker_volumes_size))
            .with_ansi_cell(docker_volumes_dir.display().to_string().dim()),
    );

    // Other artifacts (total - docker images - docker volumes)
    let other_artifacts_size =
        artifacts_size.saturating_sub(docker_images_size + docker_volumes_size);
    let other_artifacts_path = artifacts_dir.display().to_string();
    table.add_row(
        Row::new()
            .with_cell("  └─ Other Artifacts")
            .with_ansi_cell(format_size(other_artifacts_size))
            .with_ansi_cell(format!("{}/(other files)", other_artifacts_path).dim()),
    );

    print!("{}", table);

    // Total size
    let total_size = get_directory_size(&home_dir)?;
    let width = get_terminal_width().min(120);
    println!("{}", "─".repeat(width).blue());
    println!(
        "{} {}",
        "Total foc-localnet size:".blue().bold(),
        format_size(total_size).blue().bold()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_print_disk_usage() {
        // This test verifies that the function doesn't panic
        let result = print_disk_usage();
        // We expect this to work even if directories don't exist
        assert!(result.is_ok());
    }
}
