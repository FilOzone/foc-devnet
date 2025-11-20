//! Clean command implementation.
//!
//! This module handles cleaning various parts of the foc-localnet environment.

use std::fs;
use std::process::Command;
use crate::paths::{
    foc_localnet_artifacts, foc_localnet_bin, foc_localnet_curio_repo,
    foc_localnet_docker_images, foc_localnet_lotus_repo,
};

/// Clean various parts of the foc-localnet environment.
///
/// If no specific flags are provided, cleans everything.
pub fn clean_environment(
    artifacts: bool,
    docker_images: bool,
    binaries: bool,
    lotus: bool,
    curio: bool,
    clean_all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // If no specific flags provided, clean everything
    let should_clean_all = clean_all || (!artifacts && !docker_images && !binaries && !lotus && !curio);

    if should_clean_all || artifacts {
        clean_artifacts()?;
    }

    if should_clean_all || docker_images {
        clean_docker_images()?;
    }

    if should_clean_all || binaries {
        clean_binaries()?;
    }

    if should_clean_all || lotus {
        make_clean_lotus()?;
    }

    if should_clean_all || curio {
        make_clean_curio()?;
    }

    println!("✓ Cleanup completed successfully");
    Ok(())
}

/// Clean all downloaded artifacts.
fn clean_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts_dir = foc_localnet_artifacts();
    
    if artifacts_dir.exists() {
        println!("Cleaning artifacts directory: {}", artifacts_dir.display());
        fs::remove_dir_all(&artifacts_dir)?;
        println!("✓ Artifacts cleaned");
    } else {
        println!("Artifacts directory does not exist, skipping");
    }

    Ok(())
}

/// Clean all Docker images created by foc-localnet.
fn clean_docker_images() -> Result<(), Box<dyn std::error::Error>> {
    println!("Cleaning foc-localnet Docker images...");

    // Remove the builder image
    let image_name = "foc-localnet-builder:latest";
    
    let output = Command::new("docker")
        .args(["images", "-q", image_name])
        .output()?;

    if !output.stdout.is_empty() {
        println!("Removing Docker image: {}", image_name);
        let status = Command::new("docker")
            .args(["rmi", "-f", image_name])
            .status()?;

        if status.success() {
            println!("✓ Docker image removed: {}", image_name);
        } else {
            println!("⚠ Failed to remove Docker image: {}", image_name);
        }
    } else {
        println!("Docker image {} not found, skipping", image_name);
    }

    // Clean the docker images directory
    let docker_images_dir = foc_localnet_docker_images();
    if docker_images_dir.exists() {
        println!("Cleaning docker images directory: {}", docker_images_dir.display());
        fs::remove_dir_all(&docker_images_dir)?;
        println!("✓ Docker images directory cleaned");
    }

    Ok(())
}

/// Clean all built binaries.
fn clean_binaries() -> Result<(), Box<dyn std::error::Error>> {
    let bin_dir = foc_localnet_bin();
    
    if bin_dir.exists() {
        println!("Cleaning binaries directory: {}", bin_dir.display());
        
        // List binaries to be removed
        if let Ok(entries) = fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    println!("  Removing: {}", path.file_name().unwrap().to_string_lossy());
                    fs::remove_file(path)?;
                }
            }
        }
        
        println!("✓ Binaries cleaned");
    } else {
        println!("Binaries directory does not exist, skipping");
    }

    Ok(())
}

/// Run `make clean` in the Lotus repository.
fn make_clean_lotus() -> Result<(), Box<dyn std::error::Error>> {
    let lotus_repo = foc_localnet_lotus_repo();
    
    if !lotus_repo.exists() {
        println!("Lotus repository does not exist, skipping make clean");
        return Ok(());
    }

    if lotus_repo.is_symlink() {
        println!("⚠ Lotus repository is a symlink, skipping make clean (run it manually in the source directory)");
        return Ok(());
    }

    println!("Running 'make clean' in Lotus repository...");
    let status = Command::new("make")
        .arg("clean")
        .current_dir(&lotus_repo)
        .status()?;

    if status.success() {
        println!("✓ Lotus repository cleaned");
    } else {
        println!("⚠ Failed to run 'make clean' in Lotus repository");
    }

    Ok(())
}

/// Run `make clean` in the Curio repository.
fn make_clean_curio() -> Result<(), Box<dyn std::error::Error>> {
    let curio_repo = foc_localnet_curio_repo();
    
    if !curio_repo.exists() {
        println!("Curio repository does not exist, skipping make clean");
        return Ok(());
    }

    if curio_repo.is_symlink() {
        println!("⚠ Curio repository is a symlink, skipping make clean (run it manually in the source directory)");
        return Ok(());
    }

    println!("Running 'make clean' in Curio repository...");
    let status = Command::new("make")
        .arg("clean")
        .current_dir(&curio_repo)
        .status()?;

    if status.success() {
        println!("✓ Curio repository cleaned");
    } else {
        println!("⚠ Failed to run 'make clean' in Curio repository");
    }

    Ok(())
}
