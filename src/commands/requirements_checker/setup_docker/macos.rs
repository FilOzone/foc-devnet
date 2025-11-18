//! macOS-specific Docker setup utilities.

use crossterm::style::Stylize;
use std::process::Command;

/// Attempt to install Homebrew
pub fn install_homebrew() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🍺 Installing Homebrew...".blue());
    let status = Command::new("/bin/bash")
        .arg("-c")
        .arg("curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh | bash")
        .status()?;
    if status.success() {
        println!("{}", "✅ Homebrew installed successfully.".green());
        println!(
            "{}",
            "🔄 You may need to restart your terminal for Homebrew to be available in PATH."
                .yellow()
        );
        Ok(())
    } else {
        eprintln!("{}", "❌ Failed to install Homebrew.".red());
        eprintln!(
            "{}",
            "Please install Homebrew manually from https://brew.sh/".cyan()
        );
        Err("Failed to install Homebrew".into())
    }
}
