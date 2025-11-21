//! Build command implementation.
//!
//! This module handles building Filecoin projects (Lotus and Curio) in a Docker container.

pub mod repository;

use crate::config::Config;
use crate::paths::{foc_localnet_bin, foc_localnet_docker_images, foc_localnet_docker_volumes};
use crossterm::style::Stylize;
use repository::prepare_repository;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Build a project in a Docker container.
///
/// This function orchestrates the complete build process:
/// 1. Prepares the repository (clone/checkout or symlink) based on config
/// 2. Builds the project in a Docker container
pub fn build_project(project: &Project, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} Building {}...", "🔨".bold(), project);

    // Get the location configuration for this project
    let location = match project {
        Project::Lotus => &config.lotus,
        Project::Curio => &config.curio,
    };

    // Prepare the repository (clone/checkout or symlink)
    let repo_path = prepare_repository(project, location)?;

    // Build in Docker container
    let output_dir = foc_localnet_bin();
    fs::create_dir_all(&output_dir)?;

    build_in_container(
        repo_path.to_str().ok_or("Invalid repository path")?,
        output_dir.to_str().ok_or("Invalid output directory path")?,
        project,
    )?;

    println!(
        "{} {} built successfully. Binaries available in {}",
        "✓".green(),
        project,
        output_dir.display()
    );

    Ok(())
}

#[derive(Debug)]
pub enum Project {
    Lotus,
    Curio,
}

impl std::fmt::Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Project::Lotus => write!(f, "lotus"),
            Project::Curio => write!(f, "curio"),
        }
    }
}

/// Build the project in a Docker container
fn build_in_container(
    source_dir: &str,
    output_dir: &str,
    project: &Project,
) -> Result<(), Box<dyn std::error::Error>> {
    let dockerfile_dir = "docker"; // Assuming docker/Dockerfile exists

    // Build the Docker image
    let image_tag = build_builder_image(dockerfile_dir)?;

    // Run the build in container
    run_build_in_container(source_dir, output_dir, project, &image_tag)?;

    Ok(())
}

/// Build the builder Docker image.
fn build_builder_image(dockerfile_dir: &str) -> Result<String, Box<dyn std::error::Error>> {
    let image_tag = "foc-localnet-builder:latest";
    let cached_image_path = foc_localnet_docker_images().join("foc-builder.tar");

    // Check if cached image exists
    if cached_image_path.exists() {
        println!("{}", "Loading cached Docker image for builder...".bold());

        let status = Command::new("docker")
            .args(["load", "-i", &cached_image_path.to_string_lossy()])
            .status()?;

        if !status.success() {
            println!("{}", "Failed to load cached image, building from Dockerfile...".yellow());
            build_image_from_dockerfile(dockerfile_dir, image_tag)?;
        } else {
            println!("{} Loaded cached Docker image: {}", "✓".green(), image_tag);
        }
    } else {
        println!("{}", "No cached image found, building Docker image for builder...".bold());
        build_image_from_dockerfile(dockerfile_dir, image_tag)?;
    }

    Ok(image_tag.to_string())
}

/// Build Docker image from Dockerfile.
fn build_image_from_dockerfile(dockerfile_dir: &str, image_tag: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("docker")
        .args(["build", "-f", "docker/Dockerfile.foc-builder", "-t", image_tag, dockerfile_dir])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;

    if !status.success() {
        return Err("Failed to build Docker image".into());
    }

    Ok(())
}

/// Load volume mappings from a .volumes_map.toml file for a specific image.
fn load_volume_map(image_name: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let volumes_map_path = Path::new("docker").join(format!("{}.volumes_map.toml", image_name));
    
    if !volumes_map_path.exists() {
        // Return empty map if no volumes_map file exists
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(&volumes_map_path)?;
    
    #[derive(serde::Deserialize)]
    struct VolumesMap {
        volumes: HashMap<String, String>,
    }
    
    let volume_config: VolumesMap = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", volumes_map_path.display(), e))?;

    Ok(volume_config.volumes)
}

/// Run the build process inside the Docker container.
fn run_build_in_container(
    source_dir: &str,
    output_dir: &str,
    project: &Project,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} Building {} in container...", "🚀".bold(), project);

    let container_source_dir = "/workspace/source";
    let container_output_dir = "/workspace/output";

    let mut docker_run_args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "--name".to_string(),
        "foc-builder".to_string(),
        "-v".to_string(),
        format!("{}:{}", source_dir, container_source_dir),
        "-v".to_string(),
        format!("{}:{}", output_dir, container_output_dir),
    ];

    // Load and apply volume mappings for this image
    let volume_map = load_volume_map("foc-builder")?;
    if !volume_map.is_empty() {
        let volumes_dir = foc_localnet_docker_volumes();
        let image_volumes_dir = volumes_dir.join("foc-builder");
        
        for (host_subdir, container_path) in volume_map {
            let host_path = image_volumes_dir.join(&host_subdir);
            // Ensure the directory exists
            fs::create_dir_all(&host_path)?;
            docker_run_args.push("-v".to_string());
            docker_run_args.push(format!("{}:{}", host_path.display(), container_path));
        }
    }

    docker_run_args.push(image_tag.to_string());
    docker_run_args.push("/bin/bash".to_string());
    docker_run_args.push("-c".to_string());

    let build_script = match project {
        Project::Lotus => format!(
            "git config --global --add safe.directory {} && cd {} && make clean all && cp lotus lotus-miner lotus-worker {}",
            container_source_dir, container_source_dir, container_output_dir
        ),
        Project::Curio => format!(
            "git config --global --add safe.directory {} && cd {} && make clean all && cp curio {}",
            container_source_dir, container_source_dir, container_output_dir
        ),
    };

    docker_run_args.push(build_script);

    let status = Command::new("docker").args(&docker_run_args).status()?;

    if !status.success() {
        return Err(format!("Failed to build {} in container", project).into());
    }

    Ok(())
}
