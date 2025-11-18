//! Build command implementation.
//!
//! This module handles building Filecoin projects (Lotus and Curio) in a Docker container.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Build a project in a Docker container
pub fn build_project(
    project: &str,
    source_path: Option<String>,
    output_dir: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Building {}...", project);

    // Determine output directory
    let output_dir = output_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.foc-localnet/bin", home)
    });

    // Ensure output directory exists
    fs::create_dir_all(&output_dir)?;

    if let Some(path) = source_path {
        if !Path::new(&path).exists() {
            return Err(format!("Source path {} does not exist", path).into());
        }
        // Build from provided path
        build_in_container(&path, &output_dir, project)?;
    } else {
        // Clone and build
        build_from_clone(project, &output_dir)?;
    }

    println!("{} built successfully. Binaries available in {}", project, output_dir);

    Ok(())
}

/// Clone repository with latest release tag and build
fn build_from_clone(project: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let repo_url = match project {
        "lotus" => "https://github.com/filecoin-project/lotus.git",
        "curio" => "https://github.com/filecoin-project/curio.git",
        _ => return Err(format!("Unknown project: {}", project).into()),
    };

    // Create temporary directory
    let temp_dir = tempfile::tempdir()?;
    let repo_dir = temp_dir.path().join(project);

    println!("Cloning {} repository...", project);

    // Clone the repository
    let status = Command::new("git")
        .args(&["clone", repo_url, repo_dir.to_str().unwrap()])
        .status()?;

    if !status.success() {
        return Err(format!("Failed to clone {} repository", project).into());
    }

    // Get latest release tag
    let output = Command::new("git")
        .args(&["tag", "--sort=-version:refname"])
        .current_dir(&repo_dir)
        .output()?;

    if !output.status.success() {
        return Err(format!("Failed to get tags for {}", project).into());
    }

    let tags = String::from_utf8(output.stdout)?;
    let latest_tag = tags.lines().next().unwrap_or("master").trim();

    println!("Checking out latest release: {}", latest_tag);

    // Checkout the latest tag
    let status = Command::new("git")
        .args(&["checkout", latest_tag])
        .current_dir(&repo_dir)
        .status()?;

    if !status.success() {
        return Err(format!("Failed to checkout tag {} for {}", latest_tag, project).into());
    }

    // Build in container (temp_dir stays alive during this call)
    build_in_container(repo_dir.to_str().unwrap(), output_dir, project)?;

    // temp_dir drops here, cleaning up

    Ok(())
}

/// Build the project in a Docker container
fn build_in_container(
    source_dir: &str,
    output_dir: &str,
    project: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dockerfile_dir = "docker"; // Assuming docker/Dockerfile exists

    // Build the Docker image
    println!("Building Docker image for builder...");
    let image_tag = "foc-localnet-builder:latest";

    let status = Command::new("docker")
        .args(&["build", "-t", image_tag, dockerfile_dir])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;

    if !status.success() {
        return Err("Failed to build Docker image".into());
    }

    // Run the build in container
    println!("Building {} in container...", project);

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
        "lotus" => format!(
            "cd {} && make clean all && cp lotus lotus-miner lotus-worker {}",
            container_source_dir, container_output_dir
        ),
        "curio" => format!(
            "cd {} && make clean all && cp curio {}",
            container_source_dir, container_output_dir
        ),
        _ => return Err(format!("Unknown project: {}", project).into()),
    };

    docker_run_args.push(build_script);

    let status = Command::new("docker")
        .args(&docker_run_args)
        .status()?;

    if !status.success() {
        return Err(format!("Failed to build {} in container", project).into());
    }

    Ok(())
}