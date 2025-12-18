use std::process::Command;

fn main() {
    // Generate MockUSDFC.tar.gz archive
    generate_mockusdfc_archive();

    // Set build time in human readable format and as timestamp
    let utc_time = chrono::Utc::now();
    let local_time = chrono::Local::now();

    let utc_formatted = utc_time.format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let local_formatted = local_time.format("%Y-%m-%d %H:%M:%S %Z").to_string();
    let timestamp = utc_time.timestamp().to_string();

    println!("cargo:rustc-env=BUILD_TIME_UTC={}", utc_formatted);
    println!("cargo:rustc-env=BUILD_TIME_LOCAL={}", local_formatted);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);

    // Get git commit hash
    if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        if output.status.success() {
            let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_COMMIT={}", commit_hash);
        } else {
            println!("cargo:rustc-env=GIT_COMMIT=unknown");
        }
    } else {
        println!("cargo:rustc-env=GIT_COMMIT=unknown");
    }

    // Get git branch
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
    {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_BRANCH={}", branch);
        } else {
            println!("cargo:rustc-env=GIT_BRANCH=unknown");
        }
    } else {
        println!("cargo:rustc-env=GIT_BRANCH=unknown");
    }

    // Check if working directory is dirty (has uncommitted changes)
    let is_dirty = if let Ok(output) = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
    {
        output.status.success() && !output.stdout.is_empty()
    } else {
        false
    };

    println!(
        "cargo:rustc-env=GIT_DIRTY={}",
        if is_dirty { "dirty" } else { "" }
    );

    // Re-run if git info changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");

    // Re-run if MockUSDFC contract files change
    println!("cargo:rerun-if-changed=contracts/MockUSDFC/src/MockUSDFC.sol");
    println!("cargo:rerun-if-changed=contracts/MockUSDFC/script/Deploy.s.sol");
    println!("cargo:rerun-if-changed=contracts/MockUSDFC/script/Verify.s.sol");
    println!("cargo:rerun-if-changed=contracts/MockUSDFC/foundry.toml");
    println!("cargo:rerun-if-changed=contracts/MockUSDFC/remappings.txt");
}

/// Generate MockUSDFC.tar.gz archive from the MockUSDFC Foundry project
fn generate_mockusdfc_archive() {
    use std::path::Path;

    let mockusdfc_dir = Path::new("contracts/MockUSDFC");

    // Check if MockUSDFC directory exists
    if !mockusdfc_dir.exists() {
        eprintln!("Warning: MockUSDFC directory not found at contracts/MockUSDFC");
        return;
    }

    // Create tar.gz archive in contracts/ directory
    let output = Command::new("tar")
        .args(["-czf", "artifacts/MockUSDFC.tar.gz", "contracts/MockUSDFC"])
        .current_dir(".")
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                println!("cargo:warning=Created MockUSDFC.tar.gz archive");
            } else {
                eprintln!(
                    "Warning: Failed to create MockUSDFC.tar.gz: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to run tar command: {}", e);
        }
    }
}
