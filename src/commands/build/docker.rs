//! Docker operations for building projects.
//!
//! This module handles Docker image building and container execution for project builds.

use crate::docker::{
    build::build_docker_image,
    core::{get_current_gid, get_current_uid, image_exists},
};
use crate::embedded_assets;
use crate::paths::foc_localnet_docker_volumes;
use crossterm::style::Stylize;
use std::collections::HashMap;
use std::fs;

use super::Project;

/// Build the builder Docker image.
pub fn build_builder_image(dockerfile_dir: &str) -> Result<String, Box<dyn std::error::Error>> {
    let image_tag = "foc-localnet-builder:latest";

    // Check if image already exists in Docker
    if image_exists(image_tag)? {
        println!(
            "{} Docker image {} already exists, skipping build",
            "✓".green(),
            image_tag
        );
    } else {
        println!("{}", "Building Docker image for builder...".bold());
        build_image_from_dockerfile(dockerfile_dir, image_tag)?;
    }

    Ok(image_tag.to_string())
}

/// Build Docker image from Dockerfile.
pub fn build_image_from_dockerfile(
    dockerfile_dir: &str,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dockerfile_path = "docker/Dockerfile.builder";
    let output = build_docker_image(dockerfile_path, image_tag, dockerfile_dir)?;

    if !output.status.success() {
        return Err("Failed to build Docker image".into());
    }

    Ok(())
}

/// Load volume mappings from embedded volumes_map.toml file for a specific image.
pub fn load_volume_map(
    image_name: &str,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let content_bytes = embedded_assets::get_volumes_map(image_name)
        .ok_or_else(|| format!("Embedded volumes map not found for: {}", image_name))?;

    let content = std::str::from_utf8(content_bytes)
        .map_err(|e| format!("Invalid UTF-8 in volumes map for {}: {}", image_name, e))?;

    #[derive(serde::Deserialize)]
    struct VolumesMap {
        volumes: HashMap<String, String>,
    }

    let volume_config: VolumesMap = toml::from_str(content).map_err(|e| {
        format!(
            "Failed to parse embedded volumes map for {}: {}",
            image_name, e
        )
    })?;

    Ok(volume_config.volumes)
}

/// Set up the Docker run arguments for the build container.
pub fn setup_docker_run_args(
    source_dir: &str,
    output_dir: &str,
    image_tag: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
    let volume_map = load_volume_map("builder")?;
    if !volume_map.is_empty() {
        let volumes_dir = foc_localnet_docker_volumes();
        let image_volumes_dir = volumes_dir.join("builder");

        for (host_subdir, container_path) in volume_map {
            let host_path = image_volumes_dir.join(&host_subdir);
            // Ensure the directory exists
            fs::create_dir_all(&host_path)?;
            docker_run_args.push("-v".to_string());
            docker_run_args.push(format!("{}:{}", host_path.display(), container_path));
        }
    }

    // Get current user's UID and GID to run container as the same user
    let uid = get_current_uid()?;
    let gid = get_current_gid()?;

    docker_run_args.push("-u".to_string());
    docker_run_args.push(format!("{}:{}", uid, gid));

    docker_run_args.push(image_tag.to_string());
    docker_run_args.push("/bin/bash".to_string());
    docker_run_args.push("-c".to_string());

    Ok(docker_run_args)
}

/// Set up the build script for the specific project.
pub fn setup_build_script(
    project: &Project,
    container_source_dir: &str,
    container_output_dir: &str,
) -> String {
    match project {
        Project::Lotus => format!(
            r#"git config --global --add safe.directory {} && \
                cd {} && \
                make clean && \
                make 2k && \
                make lotus-shed && \
                cp lotus lotus-miner lotus-shed lotus-seed {}"#,
            container_source_dir, container_source_dir, container_output_dir
        ),
        Project::Curio => format!(
            r#"git config --global --add safe.directory {} && \
                cd {} && \
                make clean all && \
                cp curio {}"#,
            container_source_dir, container_source_dir, container_output_dir
        ),
    }
}
