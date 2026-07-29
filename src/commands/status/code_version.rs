//! # Code Version Status
//!
//! This module handles the display of code version information for the foc-devnet system.
//!
//! It provides functionality to:
//! - Display git repository information for Lotus and Curio
//! - Show source types (local, git tag, git branch, git commit)
//! - Indicate readiness status of code repositories

use crate::config::Config;
use crate::paths::foc_devnet_config;
use std::{fs, io::Error};
use tracing::info;

use super::git::{format_location_info, get_git_info, get_repo_path_from_config};

/// Print code version information in tabular format.
///
/// This function displays version information for Lotus, Curio, and Filecoin-Services
/// repositories, including their source types, current versions,
/// commit hashes, and readiness status.
///
/// # Examples
///
/// ```rust,no_run
/// use foc_devnet::commands::status::code_version::print_code_version;
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
    // Load configuration
    let config_path = foc_devnet_config();

    // Check if config exists
    if !config_path.exists() {
        return Err(Error::new(
            std::io::ErrorKind::Unsupported,
            "Configuration not initialized. Run 'foc-devnet init' first.",
        )
        .into());
    }

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

    // Get git information for Filecoin-Services
    let fc_services_repo_path =
        get_repo_path_from_config(&config.filecoin_services, "filecoin-services");
    let fc_services_git_info = get_git_info(&fc_services_repo_path)?;

    let (fc_services_source_type, fc_services_version, fc_services_commit, fc_services_status) =
        format_location_info(
            &config.filecoin_services,
            &fc_services_git_info,
            &fc_services_repo_path,
        );

    // Print log-style output
    info!(
        "Lotus: [{},{},{}] {}",
        lotus_source_type, lotus_version, lotus_commit, lotus_status
    );
    info!(
        "Curio: [{},{},{}] {}",
        curio_source_type, curio_version, curio_commit, curio_status
    );
    info!(
        "Filecoin-Services: [{},{},{}] {}",
        fc_services_source_type, fc_services_version, fc_services_commit, fc_services_status
    );
    if let Some(pdp) = &config.pdp {
        let pdp_repo_path = get_repo_path_from_config(pdp, "pdp");
        let pdp_git_info = get_git_info(&pdp_repo_path)?;
        let (pdp_source_type, pdp_version, pdp_commit, pdp_status) =
            format_location_info(pdp, &pdp_git_info, &pdp_repo_path);
        info!(
            "PDP: [{},{},{}] {}",
            pdp_source_type, pdp_version, pdp_commit, pdp_status
        );
    }

    Ok(())
}
