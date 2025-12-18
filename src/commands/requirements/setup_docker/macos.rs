//! macOS-specific Docker setup utilities.

use std::process::Command;
use tracing::{error, info, warn};

/// Attempt to install Homebrew
pub fn install_homebrew() -> Result<(), Box<dyn std::error::Error>> {
    info!("Installing Homebrew...");
    let status = Command::new("/bin/bash")
        .arg("-c")
        .arg("curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh | bash")
        .status()?;
    if status.success() {
        info!("Homebrew installed successfully.");
        warn!("You may need to restart your terminal for Homebrew to be available in PATH.");
        Ok(())
    } else {
        error!("Failed to install Homebrew.");
        info!("Please install Homebrew manually from https://brew.sh/");
        Err("Failed to install Homebrew".into())
    }
}
