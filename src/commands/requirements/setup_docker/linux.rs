//! Linux-specific Docker setup utilities.

use crossterm::style::Stylize;
use std::process::Command;

/// Check if the system is Ubuntu or Debian-based
pub fn is_ubuntu_or_debian() -> Result<bool, Box<dyn std::error::Error>> {
    // Check for /etc/os-release file
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        let is_ubuntu = content.contains("ID=ubuntu") || content.contains("ID_LIKE=ubuntu");
        let is_debian = content.contains("ID=debian") || content.contains("ID_LIKE=debian");
        Ok(is_ubuntu || is_debian)
    } else {
        // Fallback: check for apt-get
        Ok(Command::new("which")
            .arg("apt-get")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false))
    }
}

/// Install Docker on Ubuntu/Debian
pub fn install_docker_ubuntu() -> Result<(), Box<dyn std::error::Error>> {
    update_package_index()?;
    install_prerequisites()?;
    add_docker_gpg_key()?;
    add_docker_repository()?;
    update_package_index_after_repo()?;
    install_docker_ce()?;

    println!("{}", "✅ Docker CE installed successfully.".green());
    println!(
        "{}",
        "🚀 Docker service should start automatically.".yellow()
    );
    Ok(())
}

/// Update the package index.
fn update_package_index() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Update package index
    println!("{}", "🔄 Updating package index...".yellow());
    let status = Command::new("sudo").args(["apt-get", "update"]).status()?;
    if !status.success() {
        eprintln!("{}", "❌ Failed to update package index.".red());
        return Err("Failed to update package index".into());
    }
    Ok(())
}

/// Install required prerequisites.
fn install_prerequisites() -> Result<(), Box<dyn std::error::Error>> {
    // Step 2: Install prerequisites
    println!("{}", "📦 Installing prerequisites...".yellow());
    let status = Command::new("sudo")
        .args([
            "apt-get",
            "install",
            "-y",
            "ca-certificates",
            "curl",
            "gnupg",
            "lsb-release",
        ])
        .status()?;
    if !status.success() {
        eprintln!("{}", "❌ Failed to install prerequisites.".red());
        return Err("Failed to install prerequisites".into());
    }
    Ok(())
}

/// Add Docker's GPG key.
fn add_docker_gpg_key() -> Result<(), Box<dyn std::error::Error>> {
    // Step 3: Add Docker's GPG key
    println!("{}", "🔐 Adding Docker GPG key...".yellow());
    let status = Command::new("sudo")
        .args(["bash", "-c", "curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg"])
        .status()?;
    if !status.success() {
        eprintln!("{}", "❌ Failed to add Docker GPG key.".red());
        return Err("Failed to add Docker GPG key".into());
    }
    Ok(())
}

/// Add Docker repository.
fn add_docker_repository() -> Result<(), Box<dyn std::error::Error>> {
    // Step 4: Add Docker repository
    println!("{}", "📚 Adding Docker repository...".yellow());
    let status = Command::new("sudo")
        .args(["bash", "-c", "echo \"deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable\" | tee /etc/apt/sources.list.d/docker.list > /dev/null"])
        .status()?;
    if !status.success() {
        eprintln!("{}", "❌ Failed to add Docker repository.".red());
        return Err("Failed to add Docker repository".into());
    }
    Ok(())
}

/// Update package index after adding repository.
fn update_package_index_after_repo() -> Result<(), Box<dyn std::error::Error>> {
    // Step 5: Update package index again
    println!(
        "{}",
        "🔄 Updating package index with Docker repository...".yellow()
    );
    let status = Command::new("sudo").args(["apt-get", "update"]).status()?;
    if !status.success() {
        eprintln!(
            "{}",
            "❌ Failed to update package index with Docker repository.".red()
        );
        return Err("Failed to update package index".into());
    }
    Ok(())
}

/// Install Docker CE.
fn install_docker_ce() -> Result<(), Box<dyn std::error::Error>> {
    // Step 6: Install Docker
    println!("{}", "🐳 Installing Docker CE...".yellow());
    let status = Command::new("sudo")
        .args([
            "apt-get",
            "install",
            "-y",
            "docker-ce",
            "docker-ce-cli",
            "containerd.io",
        ])
        .status()?;
    if !status.success() {
        eprintln!("{}", "❌ Failed to install Docker CE.".red());
        return Err("Failed to install Docker CE".into());
    }
    Ok(())
}
