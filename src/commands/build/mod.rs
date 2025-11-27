//! Build command implementation.
//!
//! This module handles building Filecoin projects (Lotus and Curio) in a Docker container.

pub mod repository;

use crate::config::Config;
use crate::paths::{foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_logs};
use crossterm::style::Stylize;
use repository::prepare_repository;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

    // Check if image already exists in Docker
    let image_exists = Command::new("docker")
        .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
        .output()
        .map(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .any(|line| line == image_tag)
        })
        .unwrap_or(false);

    if image_exists {
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
fn build_image_from_dockerfile(
    dockerfile_dir: &str,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("docker")
        .args([
            "build",
            "--progress",
            "tty",
            "-f",
            "docker/Dockerfile.builder",
            "-t",
            image_tag,
            dockerfile_dir,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;

    if !status.success() {
        return Err("Failed to build Docker image".into());
    }

    Ok(())
}

/// Create a timestamped log file path for build logs.
fn create_build_log_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let logs_dir = foc_localnet_logs().join("build");
    fs::create_dir_all(&logs_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_path = logs_dir.join(format!("{}.log", timestamp));

    Ok(log_path)
}

/// Load volume mappings from a .volumes_map.toml file for a specific image.
fn load_volume_map(
    image_name: &str,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
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

    // Create log file for this build
    let log_path = create_build_log_path()?;
    println!(
        "{} Logs will be saved to: {}",
        "📝".bold(),
        log_path.display()
    );

    let container_source_dir = "/workspace/source";
    let container_output_dir = "/workspace/output";

    let docker_run_args = setup_docker_run_args(source_dir, output_dir, image_tag)?;
    let build_script = setup_build_script(project, container_source_dir, container_output_dir);

    execute_build_process(docker_run_args, build_script, &log_path, project)?;

    println!(
        "{} Build logs saved to: {}",
        "✓".green(),
        log_path.display()
    );

    Ok(())
}

/// Set up the Docker run arguments for the build container.
fn setup_docker_run_args(
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
    let uid_output = Command::new("id").arg("-u").output()?;
    let gid_output = Command::new("id").arg("-g").output()?;
    let uid = String::from_utf8_lossy(&uid_output.stdout)
        .trim()
        .to_string();
    let gid = String::from_utf8_lossy(&gid_output.stdout)
        .trim()
        .to_string();

    docker_run_args.push("-u".to_string());
    docker_run_args.push(format!("{}:{}", uid, gid));

    docker_run_args.push(image_tag.to_string());
    docker_run_args.push("/bin/bash".to_string());
    docker_run_args.push("-c".to_string());

    Ok(docker_run_args)
}

/// Set up the build script for the specific project.
fn setup_build_script(
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

/// Execute the build process in the Docker container.
fn execute_build_process(
    mut docker_run_args: Vec<String>,
    build_script: String,
    log_path: &Path,
    project: &Project,
) -> Result<(), Box<dyn std::error::Error>> {
    docker_run_args.push(build_script);

    // Spawn the process with piped stdout/stderr
    let mut child = Command::new("docker")
        .args(&docker_run_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Get handles to stdout and stderr
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    // Create clones of the log file for writing
    let log_file_clone = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    // Stream stdout to both console and log file
    let stdout_handle = std::thread::spawn({
        let mut log_file = log_file_clone;
        move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    println!("{}", line);
                    writeln!(log_file, "{}", line).ok();
                }
            }
        }
    });

    // Stream stderr to both console and log file
    let stderr_handle = std::thread::spawn({
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    eprintln!("{}", line);
                    writeln!(log_file, "{}", line).ok();
                }
            }
        }
    });

    // Wait for both threads to finish
    stdout_handle.join().ok();
    stderr_handle.join().ok();

    // Wait for the child process to finish
    let status = child.wait()?;

    if !status.success() {
        return Err(format!(
            "Failed to build {} in container. Check logs at: {}",
            project,
            log_path.display()
        )
        .into());
    }

    Ok(())
}
