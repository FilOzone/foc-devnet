//! FOC network metadata management.
//!
//! This module handles loading, saving, and managing network configuration
//! metadata from the FOC deployment process.

use crate::paths::foc_metadata_file;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;

/// FOC network configuration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FOCMetadata {
    /// Network name (e.g., "devnet")
    pub network_name: String,
    /// Challenge finality in epochs
    pub challenge_finality: String,
    /// Maximum proving period in epochs
    pub max_proving_period: String,
    /// Challenge window size in epochs
    pub challenge_window_size: String,
    /// Service name
    pub service_name: String,
    /// Service description
    pub service_description: String,
}

impl FOCMetadata {
    /// Load FOC metadata from the state file
    #[allow(dead_code)]
    pub fn load(run_id: &str) -> Result<Self, Box<dyn Error>> {
        let path = foc_metadata_file(run_id);
        if !path.exists() {
            return Err("FOC metadata file not found".into());
        }
        let content = fs::read_to_string(&path)?;
        let metadata: FOCMetadata = serde_json::from_str(&content)?;
        Ok(metadata)
    }

    /// Save FOC metadata to the state file
    pub fn save(&self, run_id: &str) -> Result<(), Box<dyn Error>> {
        let path = foc_metadata_file(run_id);
        // Ensure the state directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        Ok(())
    }
}
