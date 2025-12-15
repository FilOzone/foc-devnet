//! Provider ID state management.

use crate::paths::pdp_sp_0_provider_id_file;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;

/// Provider ID information stored in state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderIdInfo {
    pub provider_id: u64,
    pub provider_address: String,
    pub payee_address: String,
}

impl ProviderIdInfo {
    /// Load provider ID from state file
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path = pdp_sp_0_provider_id_file();
        if !path.exists() {
            return Err("Provider ID file not found".into());
        }
        let content = fs::read_to_string(&path)?;
        let info: ProviderIdInfo = serde_json::from_str(&content)?;
        Ok(info)
    }

    /// Save provider ID to state file
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let path = pdp_sp_0_provider_id_file();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self)?;
        fs::write(&path, json)?;

        Ok(())
    }

    /// Check if provider ID file exists
    pub fn exists() -> bool {
        pdp_sp_0_provider_id_file().exists()
    }
}
