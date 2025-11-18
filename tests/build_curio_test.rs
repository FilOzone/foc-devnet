use std::process::Command;

/// Test that the Curio build command works end-to-end
#[test]
fn test_curio_build_integration() {
    // Skip this test if Docker is not available
    if !docker_available() {
        println!("Skipping Curio build test: Docker not available");
        return;
    }

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

    assert!(help_output.status.success(), "Build curio help command failed");
    let help_text = String::from_utf8_lossy(&help_output.stdout);
    assert!(help_text.contains("Curio"), "Help text doesn't mention Curio");

    // Test that invalid path fails gracefully
    let invalid_output = Command::new("./target/debug/foc-localnet")
        .args(["build", "curio", "/nonexistent/path"])
        .output()
        .expect("Failed to run build with invalid path");

    assert!(!invalid_output.status.success(), "Expected command to fail with nonexistent path");

    // Test Docker image building (this is the core functionality we want to test)
    let build_status = Command::new("docker")
        .args(["build", "-t", "foc-localnet-builder-test", "./docker"])
        .status()
        .expect("Failed to build Docker image");

    assert!(build_status.success(), "Failed to build Docker builder image");

    // Verify the image was created
    let images_output = Command::new("docker")
        .args(["images", "foc-localnet-builder-test", "--format", "{{.Repository}}:{{.Tag}}"])
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

/// Helper function to check if Docker is available
fn docker_available() -> bool {
    Command::new("docker")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}