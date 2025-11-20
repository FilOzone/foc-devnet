//! Init command implementation.
//!
//! This module handles comprehensive initialization of foc-localnet including:
//! - Creating all necessary directories
//! - Generating default configuration
//! - Setting up PATH variables in shell configs
//! - Downloading required artifacts
//! - Building and caching Docker images

use dirs;
use downloader::Downloader;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::debug;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::Config;
use crate::paths::{
    foc_localnet_artifacts, foc_localnet_bin, foc_localnet_code, foc_localnet_config,
    foc_localnet_docker_images, foc_localnet_home, foc_localnet_logs, foc_localnet_state,
    foc_localnet_tmp,
};

/// Initialize foc-localnet comprehensively.
///
/// This command performs complete initialization:
/// 1. Creates all necessary directories
/// 2. Generates default config.toml
/// 3. Sets up PATH variables in shell configs
/// 4. Downloads required artifacts
/// 5. Builds and caches Docker images
pub fn init_environment() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing foc-localnet environment...");

    // Create all necessary directories
    create_directories()?;

    // Generate default configuration
    generate_default_config()?;

    // Set up PATH variables
    setup_path_variables()?;

    // Download required artifacts
    download_artifacts()?;

    // Build and cache Docker images
    build_and_cache_docker_images()?;

    println!("✓ Initialization completed successfully");
    println!(
        "You may need to restart your shell or run 'source ~/.bashrc' (or ~/.zshrc) to use the updated PATH"
    );
    Ok(())
}

/// Create all necessary directories for foc-localnet.
fn create_directories() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating necessary directories...");

    let directories = vec![
        foc_localnet_home(),
        foc_localnet_logs(),
        foc_localnet_bin(),
        foc_localnet_state(),
        foc_localnet_code(),
        foc_localnet_tmp(),
        foc_localnet_artifacts(),
        foc_localnet_docker_images(),
    ];

    for dir in directories {
        if !dir.exists() {
            debug!("Creating directory: {:?}", dir);
            fs::create_dir_all(&dir)?;
            println!("  ✓ Created: {}", dir.display());
        } else {
            println!("  ✓ Exists: {}", dir.display());
        }
    }

    Ok(())
}

/// Generate default configuration file if it doesn't exist.
fn generate_default_config() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = foc_localnet_config();

    if config_path.exists() {
        println!("  ✓ Config file already exists: {}", config_path.display());
        return Ok(());
    }

    debug!("Generating default config: {:?}", config_path);
    let default_config = toml::to_string(&Config::default())
        .map_err(|e| format!("Failed to serialize default config: {}", e))?;

    fs::write(&config_path, default_config)?;
    println!("  ✓ Created default config: {}", config_path.display());

    Ok(())
}

/// Set up PATH variables in shell configuration files.
fn setup_path_variables() -> Result<(), Box<dyn std::error::Error>> {
    let bin_path = foc_localnet_bin();
    let bin_path_str = bin_path.to_string_lossy().to_string();

    if is_path_in_env(&bin_path_str) {
        println!("  ✓ PATH already includes: {}", bin_path_str);
        return Ok(());
    }

    println!("Setting up PATH variables...");

    if let Some(home) = dirs::home_dir() {
        let bashrc = home.join(".bashrc");
        if let Err(e) = add_path_to_shell_config(&bashrc, &bin_path_str) {
            println!("  ⚠ Failed to update .bashrc: {}", e);
        } else {
            println!("  ✓ Updated .bashrc");
        }

        let zshrc = home.join(".zshrc");
        if let Err(e) = add_path_to_shell_config(&zshrc, &bin_path_str) {
            println!("  ⚠ Failed to update .zshrc: {}", e);
        } else {
            println!("  ✓ Updated .zshrc");
        }
    }

    Ok(())
}

/// Check if the given path is already in the PATH environment variable.
fn is_path_in_env(bin_path: &str) -> bool {
    let current_path = env::var("PATH").unwrap_or_default();
    current_path.split(':').any(|p| p == bin_path)
}

/// Add the bin path to a shell configuration file if not already present.
fn add_path_to_shell_config(
    config_path: &Path,
    bin_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !config_path.exists() {
        return Ok(());
    }

    let mut content = fs::read_to_string(config_path)?;
    let marker = "# foc-localnet PATH addition";

    if content.contains(marker) {
        return Ok(());
    }

    content.push_str(&format!(
        "\n{} \nexport PATH=\"$PATH:{}\"\n",
        marker, bin_path
    ));
    fs::write(config_path, content)?;
    Ok(())
}

/// Build and cache Docker images.
fn build_and_cache_docker_images() -> Result<(), Box<dyn std::error::Error>> {
    println!("Building and caching Docker images...");

    // Ensure the docker images directory exists
    let images_dir = foc_localnet_docker_images();
    fs::create_dir_all(&images_dir)?;

    // Find all Dockerfile files in the docker directory
    let docker_dir = Path::new("docker");
    if !docker_dir.exists() {
        println!("  ⚠ docker/ directory not found, skipping Docker image building");
        return Ok(());
    }

    let dockerfiles = find_dockerfiles(docker_dir)?;

    if dockerfiles.is_empty() {
        println!("  No Dockerfile files found in docker/ directory");
        return Ok(());
    }

    println!("  Found {} Dockerfile(s) to build:", dockerfiles.len());

    for dockerfile in dockerfiles {
        let name = extract_name(&dockerfile)?;

        // Build the Docker image
        build_docker_image(&dockerfile, &name)?;

        // Save the image as a tar file
        save_docker_image(&name, &images_dir)?;
    }

    println!("  ✓ Docker images built and cached");
    Ok(())
}

/// Find all files named Dockerfile or Dockerfile.<name> in the given directory.
fn find_dockerfiles(dir: &Path) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut dockerfiles = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename == "Dockerfile" || filename.starts_with("Dockerfile.") {
                    dockerfiles.push(path);
                }
            }
        }
    }

    Ok(dockerfiles)
}

/// Extract the name from a Dockerfile.<name> path.
/// Special case: plain "Dockerfile" becomes "builder".
fn extract_name(dockerfile_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let filename = dockerfile_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid dockerfile path")?;

    if filename == "Dockerfile" {
        Ok("builder".to_string())
    } else if let Some(name) = filename.strip_prefix("Dockerfile.") {
        Ok(name.to_string())
    } else {
        Err(format!("Invalid dockerfile name: {}", filename).into())
    }
}

/// Build a Docker image from the given Dockerfile.
fn build_docker_image(
    dockerfile_path: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let image_tag = format!("foc-localnet-{}", name);
    let dockerfile_dir = dockerfile_path.parent().unwrap_or(Path::new("."));

    println!(
        "    Building Docker image: {} from {}",
        image_tag,
        dockerfile_path.display()
    );

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Building Docker image: {}", image_tag));

    let status = Command::new("docker")
        .args([
            "build",
            "-f",
            &dockerfile_path.to_string_lossy(),
            "-t",
            &image_tag,
            &dockerfile_dir.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        pb.finish_with_message(format!("❌ Failed to build Docker image: {}", image_tag));
        return Err(format!("Failed to build Docker image: {}", image_tag).into());
    }

    pb.finish_with_message(format!("✓ Built image: {}", image_tag));
    Ok(())
}

/// Save a Docker image as a tar file.
fn save_docker_image(name: &str, images_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let image_tag = format!("foc-localnet-{}", name);
    let tar_path = images_dir.join(format!("{}.tar", name));

    println!("    Saving image to: {}", tar_path.display());

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Saving Docker image: {}", name));

    let status = Command::new("docker")
        .args(["save", "-o", &tar_path.to_string_lossy(), &image_tag])
        .status()?;

    if !status.success() {
        pb.finish_with_message(format!("❌ Failed to save Docker image: {}", image_tag));
        return Err(format!("Failed to save Docker image: {}", image_tag).into());
    }

    pb.finish_with_message(format!("✓ Saved image: {}", tar_path.display()));
    Ok(())
}

/// Download required artifacts for foc-localnet.
///
/// This function downloads Yugabyte database and extracts it to the
/// artifacts directory. It reads the download URL from the configuration.
fn download_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading artifacts...");

    // Load configuration
    let config_path = foc_localnet_config();
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Ensure artifacts directory exists
    let artifacts_dir = foc_localnet_artifacts();
    fs::create_dir_all(&artifacts_dir)?;

    // Download Yugabyte
    download_yugabyte(&config.yugabyte_download_url, &artifacts_dir)?;

    println!("  ✓ Artifacts downloaded successfully.");
    Ok(())
}

/// Downloads and extracts Yugabyte database.
///
/// Downloads the Yugabyte tarball from the given URL and extracts it
/// to the specified directory.
fn download_yugabyte(url: &str, artifacts_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Extract filename from URL
    let filename = url
        .split('/')
        .last()
        .ok_or("Invalid URL: no filename")?;
    let tarball_path = artifacts_dir.join(filename);

    // Create progress bar
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Downloading Yugabyte from {}...", url));

    // Download the tarball using downloader
    let mut downloader = Downloader::builder()
        .download_folder(artifacts_dir)
        .build()?;
    
    let dl = downloader::Download::new(url).file_name(Path::new(&filename));
    downloader.download(&[dl])?;

    pb.finish_with_message("✓ Downloaded Yugabyte");

    // Clean the yugabyte directory if it exists
    let yugabyte_dir = artifacts_dir.join("yugabyte");
    if yugabyte_dir.exists() {
        fs::remove_dir_all(&yugabyte_dir)?;
    }

    // Extract the tarball
    let pb_extract = ProgressBar::new_spinner();
    pb_extract.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb_extract.set_message("Extracting Yugabyte...");

    let status = Command::new("tar")
        .args(&["xfz", &tarball_path.to_string_lossy()])
        .current_dir(artifacts_dir)
        .status()?;

    if !status.success() {
        pb_extract.finish_with_message("❌ Failed to extract Yugabyte");
        return Err(format!("Failed to extract Yugabyte tarball").into());
    }

    // Find the extracted directory and rename it to "yugabyte"
    let mut extracted_dir = None;
    for entry in fs::read_dir(artifacts_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("yugabyte-") {
                    extracted_dir = Some(path);
                    break;
                }
            }
        }
    }

    if let Some(extracted) = extracted_dir {
        fs::rename(&extracted, &yugabyte_dir)?;
    } else {
        return Err("Could not find extracted yugabyte directory".into());
    }

    pb_extract.finish_with_message("✓ Extracted Yugabyte");

    println!("  ✓ Yugabyte downloaded and installed successfully.");
    Ok(())
}
