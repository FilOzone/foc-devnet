//! Build command implementation.
//!
//! This module handles building Filecoin projects (Lotus and Curio) in a Docker container.

pub mod repository;

use crate::config::Config;
use crate::paths::foc_localnet_bin;
use crossterm::style::Stylize;
use repository::prepare_repository;
use std::fs;
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
    println!("{}", "Building Docker image for builder...".bold());
    let image_tag = "foc-localnet-builder:latest";

    let status = Command::new("docker")
        .args(["build", "-t", image_tag, dockerfile_dir])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;

    if !status.success() {
        return Err("Failed to build Docker image".into());
    }

    Ok(image_tag.to_string())
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
        "-v".to_string(),
        format!("{}:{}", source_dir, container_source_dir),
        "-v".to_string(),
        format!("{}:{}", output_dir, container_output_dir),
        image_tag.to_string(),
        "/bin/bash".to_string(),
        "-c".to_string(),
    ];

    let build_script = match project {
        Project::Lotus => format!(
            "git config --global --add safe.directory {} && cd {} && make clean all && cp lotus lotus-miner lotus-worker {}",
            container_source_dir, container_source_dir, container_output_dir
        ),
        Project::Curio => format!(
            "git config --global --add safe.directory {} && cd {} && git checkout pdpv0 && make clean all && cp curio {}",
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
