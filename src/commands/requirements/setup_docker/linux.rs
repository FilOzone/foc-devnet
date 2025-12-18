//! Linux-specific Docker setup utilities.

use std::process::Command;
use tracing::{error, info, warn};

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

    info!("Docker CE installed successfully.");
    warn!("Docker service should start automatically.");
    Ok(())
}

/// Update the package index.
fn update_package_index() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Update package index
    info!("Updating package index...");
    let status = Command::new("sudo").args(["apt-get", "update"]).status()?;
    if !status.success() {
        error!("Failed to update package index.");
        return Err("Failed to update package index".into());
    }
    Ok(())
}

/// Install required prerequisites.
fn install_prerequisites() -> Result<(), Box<dyn std::error::Error>> {
    // Step 2: Install prerequisites
    info!("Installing prerequisites...");
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
        error!("Failed to install prerequisites.");
        return Err("Failed to install prerequisites".into());
    }
    Ok(())
}

/// Add Docker's GPG key.
fn add_docker_gpg_key() -> Result<(), Box<dyn std::error::Error>> {
    // Step 3: Add Docker's GPG key
    info!("Adding Docker GPG key...");
    let status = Command::new("sudo")
        .args(["bash", "-c", "curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg"])
        .status()?;
    if !status.success() {
        error!("Failed to add Docker GPG key.");
        return Err("Failed to add Docker GPG key".into());
    }
    Ok(())
}

/// Add Docker repository.
fn add_docker_repository() -> Result<(), Box<dyn std::error::Error>> {
    // Step 4: Add Docker repository
    info!("Adding Docker repository...");
    let status = Command::new("sudo")
        .args(["bash", "-c", "echo \"deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable\" | tee /etc/apt/sources.list.d/docker.list > /dev/null"])
        .status()?;
    if !status.success() {
        error!("Failed to add Docker repository.");
        return Err("Failed to add Docker repository".into());
    }
    Ok(())
}

/// Update package index after adding repository.
fn update_package_index_after_repo() -> Result<(), Box<dyn std::error::Error>> {
    // Step 5: Update package index again
    info!("Updating package index with Docker repository...");
    let status = Command::new("sudo").args(["apt-get", "update"]).status()?;
    if !status.success() {
        error!("Failed to update package index with Docker repository.");
        return Err("Failed to update package index".into());
    }
    Ok(())
}

/// Install Docker CE.
fn install_docker_ce() -> Result<(), Box<dyn std::error::Error>> {
    // Step 6: Install Docker
    info!("Installing Docker CE...");
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
        error!("Failed to install Docker CE.");
        return Err("Failed to install Docker CE".into());
    }
    Ok(())
}
