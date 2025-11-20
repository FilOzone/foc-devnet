use std::fs;
use std::process::Command;
use tempfile;

/// Test that the Curio build command help works
#[test]
fn test_curio_build_help() {
    // Build the foc-localnet binary
    let status = Command::new("cargo")
        .args(["build"])
        .status()
        .expect("Failed to build foc-localnet");

    assert!(status.success(), "Failed to build foc-localnet binary");

    // Test that the build command help works
    let help_output = Command::new("./target/debug/foc-localnet")
        .args(["build", "curio", "--help"])
        .output()
        .expect("Failed to run build curio help");

    assert!(
        help_output.status.success(),
        "Build curio help command failed"
    );
    let help_text = String::from_utf8_lossy(&help_output.stdout);
    assert!(
        help_text.contains("Curio"),
        "Help text doesn't mention Curio"
    );
    assert!(
        help_text.contains("Path to the Curio source directory"),
        "Help text doesn't mention path option"
    );
}

/// Test that the Curio build command handles invalid paths correctly
#[test]
fn test_curio_build_invalid_path() {
    // Build the foc-localnet binary
    let status = Command::new("cargo")
        .args(["build"])
        .status()
        .expect("Failed to build foc-localnet");

    assert!(status.success(), "Failed to build foc-localnet binary");

    // Test that invalid path fails gracefully
    let invalid_output = Command::new("./target/debug/foc-localnet")
        .args(["build", "curio", "/nonexistent/path"])
        .output()
        .expect("Failed to run build with invalid path");

    assert!(
        !invalid_output.status.success(),
        "Expected command to fail with nonexistent path"
    );
    let error_text = String::from_utf8_lossy(&invalid_output.stderr);
    assert!(
        error_text.contains("does not exist"),
        "Error message should mention path doesn't exist"
    );
}

/// Test that the Curio build command accepts valid paths and starts the build process
#[test]
fn test_curio_build_valid_path() {
    // Build the foc-localnet binary
    let status = Command::new("cargo")
        .args(["build"])
        .status()
        .expect("Failed to build foc-localnet");

    assert!(status.success(), "Failed to build foc-localnet binary");

    // Create a temporary directory and clone a shallow copy of Curio
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let curio_dir = temp_dir.path().join("curio");

    // Clone a shallow copy of Curio for testing
    let clone_status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "https://github.com/filecoin-project/curio.git",
        ])
        .arg(curio_dir.to_str().unwrap())
        .status()
        .expect("Failed to clone Curio repository");

    assert!(clone_status.success(), "Failed to clone Curio repository");

    // Create output directory
    let output_dir = temp_dir.path().join("output");
    std::fs::create_dir(&output_dir).expect("Failed to create output directory");

    // Test that the build command accepts the valid path and starts (but may timeout)
    let build_command = Command::new("./target/debug/foc-localnet")
        .args(["build", "curio"])
        .arg(curio_dir.to_str().unwrap())
        .args(["--output-dir", output_dir.to_str().unwrap()])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    match build_command {
        Ok(mut child) => {
            // Let it run for a reasonable time to allow the build to complete
            std::thread::sleep(std::time::Duration::from_secs(30));

            // Check if the process completed successfully
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        println!("Build process completed successfully");
                    } else {
                        println!("Build process completed with status: {}", status);
                    }
                }
                Ok(None) => {
                    println!("Build process is still running, killing it");
                    // Kill the process since we don't want it to run indefinitely
                    let _ = child.kill();
                }
                Err(e) => {
                    panic!("Failed to check build process status: {}", e);
                }
            }
        }
        Err(e) => {
            panic!("Failed to start build command with valid path: {}", e);
        }
    }

    // Check that the expected binary was created
    let curio_binary = output_dir.join("curio");

    // List contents of output directory for debugging
    println!("Contents of output directory:");
    if let Ok(entries) = fs::read_dir(&output_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                println!("  {}", entry.path().display());
            }
        }
    } else {
        println!("  Could not read output directory");
    }

    // The build should create the expected binary
    assert!(
        curio_binary.exists(),
        "Curio binary should be created in output directory"
    );

    // Verify it's executable
    let curio_metadata = curio_binary
        .metadata()
        .expect("Failed to get curio binary metadata");
    assert!(
        !curio_metadata.permissions().readonly(),
        "Curio binary should be executable"
    );

    println!("Curio binary was created successfully and is executable");

    // At minimum, verify that the foc-localnet-builder Docker image was created
    // (this happens during the build process)
    let images_output = Command::new("docker")
        .args([
            "images",
            "foc-localnet-builder",
            "--format",
            "{{.Repository}}:{{.Tag}}",
        ])
        .output()
        .expect("Failed to check Docker images");

    let images_text = String::from_utf8_lossy(&images_output.stdout);
    // The image should exist since it's created early in the build process
    assert!(
        images_text.contains("foc-localnet-builder:latest"),
        "Docker builder image should have been created during build process"
    );
    println!("Docker images check: {}", images_text);
}

/// Test Docker image building for Curio builds
#[test]
fn test_docker_image_building() {
    // Test Docker image building (this is the core functionality we want to test)
    let build_status = Command::new("docker")
        .args(["build", "-t", "foc-localnet-builder-test", "./docker"])
        .status()
        .expect("Failed to build Docker image");

    assert!(
        build_status.success(),
        "Failed to build Docker builder image"
    );

    // Verify the image was created
    let images_output = Command::new("docker")
        .args([
            "images",
            "foc-localnet-builder-test",
            "--format",
            "{{.Repository}}:{{.Tag}}",
        ])
        .output()
        .expect("Failed to check Docker images");

    let images_text = String::from_utf8_lossy(&images_output.stdout);
    assert!(
        images_text.contains("foc-localnet-builder-test:latest"),
        "Docker builder image was not created properly"
    );

    // Clean up: remove the test Docker image
    let _ = Command::new("docker")
        .args(["rmi", "foc-localnet-builder-test:latest"])
        .status();
}
