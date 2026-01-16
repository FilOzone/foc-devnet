//! Run ID persistence for saving and loading the current run ID.
//!
//! This module handles saving the current run ID to ~/.foc-devnet/state/current_runid.json
//! and loading it when needed for stop/status commands.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Structure for storing run ID metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct RunIdMetadata {
    /// The current run ID
    pub run_id: String,
    /// Timestamp when the run was started (ISO 8601)
    pub started_at: String,
}

/// Get the path to the current run ID file
fn current_run_id_file() -> PathBuf {
    crate::paths::foc_devnet_state().join("current_runid.json")
}

/// Save the current run ID to persistent storage.
///
/// # Arguments
/// * `run_id` - The run ID to save
///
/// # Returns
/// Ok(()) on success, error on failure
///
/// # Example
/// ```no_run
/// save_current_run_id("251203-1246-thirsty-wolf")?;
/// ```
pub fn save_current_run_id(run_id: &str) -> Result<(), Box<dyn Error>> {
    let state_dir = crate::paths::foc_devnet_state();
    fs::create_dir_all(&state_dir)?;

    let metadata = RunIdMetadata {
        run_id: run_id.to_string(),
        started_at: chrono::Local::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&metadata)?;
    let file_path = current_run_id_file();
    fs::write(&file_path, json)?;

    Ok(())
}

/// Load the current run ID from persistent storage.
///
/// # Returns
/// The run ID on success, error if file doesn't exist or can't be parsed
///
/// # Example
/// ```no_run
/// let run_id = load_current_run_id()?;
/// println!("Current run: {}", run_id);
/// ```
pub fn load_current_run_id() -> Result<String, Box<dyn Error>> {
    let file_path = current_run_id_file();

    if !file_path.exists() {
        return Err(
            "No current run ID found. Start a cluster first with 'foc-devnet start'".into(),
        );
    }

    let contents = fs::read_to_string(&file_path)?;
    let metadata: RunIdMetadata = serde_json::from_str(&contents)?;

    Ok(metadata.run_id)
}

/// Delete the current run ID file.
///
/// This should be called after successfully stopping a cluster.
///
/// # Returns
/// Ok(()) on success (including if file doesn't exist), error on failure
pub fn delete_current_run_id() -> Result<(), Box<dyn Error>> {
    let file_path = current_run_id_file();

    if file_path.exists() {
        fs::remove_file(&file_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_run_id() {
        let test_run_id = "251203-1246-test-wolf";

        // Save
        save_current_run_id(test_run_id).unwrap();

        // Load
        let loaded_id = load_current_run_id().unwrap();
        assert_eq!(loaded_id, test_run_id);

        // Clean up
        delete_current_run_id().unwrap();

        // Verify deletion
        assert!(load_current_run_id().is_err());
    }
}
