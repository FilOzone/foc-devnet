use crate::paths::{foc_localnet_run_dir, foc_localnet_run_log_file, foc_localnet_state_latest};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes the logging system.
///
/// This sets up two layers:
/// 1. A stdout layer with ANSI colors for terminal output.
/// 2. A file layer without ANSI colors for the execution log.
///
/// It also updates the `state/latest` symlink to point to the current run directory.
pub fn init_logging(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = foc_localnet_run_dir(run_id);
    fs::create_dir_all(&run_dir)?;

    let log_file_path = foc_localnet_run_log_file(run_id);
    let log_file = fs::File::create(log_file_path)?;

    let file_layer = fmt::layer().with_ansi(false).with_writer(log_file);

    let stdout_layer = fmt::layer()
        .with_ansi(true)
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .without_time();

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with(file_layer)
        .with(stdout_layer)
        .init();

    update_latest_symlink(&run_dir)?;
    Ok(())
}

fn update_latest_symlink(run_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let latest = foc_localnet_state_latest();

    // Remove existing symlink or directory if it exists
    if latest.exists() || latest.is_symlink() {
        if latest.is_symlink() || latest.is_file() {
            fs::remove_file(&latest)?;
        } else if latest.is_dir() {
            fs::remove_dir_all(&latest)?;
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = latest.parent() {
        fs::create_dir_all(parent)?;
    }

    symlink(run_dir, latest)?;
    Ok(())
}
