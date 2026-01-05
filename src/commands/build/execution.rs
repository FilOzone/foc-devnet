//! Container execution for build processes.
//!
//! This module handles running build processes inside Docker containers with logging.

use super::docker;
use super::Project;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{error, info};

/// Run the build process inside the Docker container.
pub fn run_build_in_container(
    source_dir: &str,
    output_dir: &str,
    project: &Project,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Building {} in container...", project);

    let container_source_dir = "/workspace/source";
    let container_output_dir = "/workspace/output";

    let docker_run_args =
        docker::setup_docker_run_args(source_dir, output_dir, image_tag, project)?;
    let build_script =
        docker::setup_build_script(project, container_source_dir, container_output_dir);

    execute_build_process(docker_run_args, build_script, &Path::new(""), project)?;

    info!("Build completed for {}", project);

    Ok(())
}

/// Execute the build process in the Docker container.
pub fn execute_build_process(
    mut docker_run_args: Vec<String>,
    build_script: String,
    _log_path: &Path,
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

    // Stream stdout to console
    let stdout_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            info!("{}", line);
        }
    });

    // Stream stderr to console
    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            error!("{}", line);
        }
    });

    // Wait for both threads to finish
    stdout_handle.join().ok();
    stderr_handle.join().ok();

    // Wait for the child process to finish
    let status = child.wait()?;

    if !status.success() {
        return Err(format!(
            "Failed to build {} in container.",
            project
        )
        .into());
    }

    Ok(())
}
