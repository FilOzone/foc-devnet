use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Get the path to the poison file
fn poison_file_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let state_dir = dirs::home_dir()
        .ok_or("Could not determine home directory")?
        .join(".foc-localnet")
        .join("state");

    // Ensure state directory exists
    fs::create_dir_all(&state_dir)?;

    Ok(state_dir.join(".poison"))
}

/// Check if a poison file exists and attempt recovery if found
pub fn check_and_recover_poison() -> Result<(), Box<dyn std::error::Error>> {
    let poison_path = poison_file_path()?;

    if poison_path.exists() {
        warn!("Poison file detected at: {}", poison_path.display());

        // Read and display the poison file contents
        match fs::read_to_string(&poison_path) {
            Ok(contents) => {
                warn!("Poison file contents:");
                for line in contents.lines() {
                    if !line.trim().is_empty() {
                        warn!("  {}", line);
                    }
                }
            }
            Err(e) => {
                warn!("Could not read poison file contents: {}", e);
            }
        }

        warn!("This indicates a previous command may have failed. Attempting recovery...");

        // For now, just warn about the poison file
        // TODO: Implement actual recovery logic when more details are available
        warn!("Recovery logic not yet implemented. Please check system state manually.");
        warn!("You may need to manually clean up any running containers or inconsistent state.");

        // Remove the poison file after warning
        fs::remove_file(&poison_path)?;
        info!("Poison file removed. Proceeding with caution.");
    }

    Ok(())
}

/// Create the poison file to mark the start of a potentially dangerous operation
pub fn create_poison(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let poison_path = poison_file_path()?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let log_entry = format!("{} invoked at {}\n", command, timestamp);

    fs::write(&poison_path, log_entry)?;
    debug!("Poison file created at: {}", poison_path.display());
    Ok(())
}

/// Remove the poison file after successful completion
/// Does not do anything if the poison file does not exist.
pub fn remove_poison() -> Result<(), Box<dyn std::error::Error>> {
    let poison_path = poison_file_path()?;

    if poison_path.exists() {
        fs::remove_file(&poison_path)?;
        debug!("Poison file removed - operation completed successfully");
    }

    Ok(())
}
