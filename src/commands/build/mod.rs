//! Build command implementation.
//!
//! This module handles building Filecoin projects (Lotus and Curio) in a Docker container.

pub mod docker;
pub mod exec;
pub mod logging;
pub mod repository;

use crate::config::Config;
use crate::paths::foc_devnet_bin;
use repository::prepare_repository;
use std::fs;
use tracing::info;

use self::docker::build_builder_image;
use self::exec::run_build_in_container;

/// Build a project in a Docker container.
///
/// This function orchestrates the complete build process:
/// 1. Prepares the repository (clone/checkout or symlink) based on config
/// 2. Builds the project in a Docker container
pub fn build_project(project: &Project, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    info!("Building {}...", project);

    // Get the location configuration for this project
    let location = match project {
        Project::Lotus => &config.lotus,
        Project::Curio => &config.curio,
    };

    // Prepare the repository (clone/checkout or symlink)
    let repo_path = prepare_repository(project, location)?;

    // Build in Docker container
    let output_dir = foc_devnet_bin();
    fs::create_dir_all(&output_dir)?;

    build_in_container(
        repo_path.to_str().ok_or("Invalid repository path")?,
        output_dir.to_str().ok_or("Invalid output directory path")?,
        project,
    )?;

    info!(
        "{} built successfully. Binaries available in {}",
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
