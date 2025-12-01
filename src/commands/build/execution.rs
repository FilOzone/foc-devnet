//! Container execution for build processes.
//!
//! This module handles running build processes inside Docker containers with logging.

use super::docker;
use super::logging;
use super::Project;
use crossterm::style::Stylize;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Run the build process inside the Docker container.
pub fn run_build_in_container(
    source_dir: &str,
    output_dir: &str,
    project: &Project,
    image_tag: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} Building {} in container...", "🚀".bold(), project);

    // Create log file for this build
    let log_path = logging::create_build_log_path()?;
    println!(
        "{} Logs will be saved to: {}",
        "📝".bold(),
        log_path.display()
    );

    let container_source_dir = "/workspace/source";
    let container_output_dir = "/workspace/output";

    let docker_run_args = docker::setup_docker_run_args(source_dir, output_dir, image_tag)?;
    let build_script =
        docker::setup_build_script(project, container_source_dir, container_output_dir);

    execute_build_process(docker_run_args, build_script, &log_path, project)?;

    println!(
        "{} Build logs saved to: {}",
        "✓".green(),
        log_path.display()
    );

    Ok(())
}

/// Execute the build process in the Docker container.
pub fn execute_build_process(
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
