//! Build logging utilities.
//!
//! This module handles creation and management of build log files.

use crate::paths::foc_devnet_logs;
use std::fs;
use std::path::PathBuf;

/// Create a timestamped log file path for build logs.
pub fn create_build_log_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let logs_dir = foc_devnet_logs().join("build");
    fs::create_dir_all(&logs_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_path = logs_dir.join(format!("{}.log", timestamp));

    Ok(log_path)
}
