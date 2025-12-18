//! Contract addresses management for FOC deployment.
//!
//! This module handles loading, saving, and managing contract addresses
//! that are deployed during the FOC deployment process.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;

use crate::paths::contract_addresses_file;

/// Contract addresses and deployment information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ContractAddresses {
    /// Standard contracts (multicall, USDFC, etc.)
    pub contracts: std::collections::HashMap<String, String>,
    /// FOC service contracts
    pub foc_contracts: std::collections::HashMap<String, String>,
    /// FilBeam controller address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filbeam_controller: Option<String>,
    /// FilBeam beneficiary address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filbeam_beneficiary: Option<String>,
}

impl ContractAddresses {
    /// Load contract addresses from the state file
    ///
    /// # Returns
    /// The loaded contract addresses or an error if the file doesn't exist or is invalid
    ///
    /// # Examples
    /// ```rust
    /// use crate::commands::start::foc_deploy::contract_addresses::ContractAddresses;
    ///
    /// let addresses = ContractAddresses::load()?;
    /// println!("Contracts: {:?}", addresses.contracts);
    /// ```
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path = contract_addresses_file();
        if !path.exists() {
            return Err("Contract addresses file not found".into());
        }
        let content = fs::read_to_string(&path)?;
        let addresses: ContractAddresses = serde_json::from_str(&content)?;
        Ok(addresses)
    }

    /// Save contract addresses to the state file
    ///
    /// # Examples
    /// ```rust
    /// use crate::commands::start::foc_deploy::contract_addresses::ContractAddresses;
    /// use std::collections::HashMap;
    ///
    /// let mut contracts = HashMap::new();
    /// contracts.insert("multicall".to_string(), "0x123...".to_string());
    ///
    /// let addresses = ContractAddresses {
    ///     contracts,
    ///     foc_contracts: HashMap::new(),
    /// };
    /// addresses.save()?;
    /// ```
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let path = contract_addresses_file();
        // Ensure the state directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        Ok(())
    }
}

