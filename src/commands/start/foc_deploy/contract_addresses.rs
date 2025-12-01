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
pub struct ContractAddresses {
    /// Global FIL faucet address (BLS/f3)
    pub global_fil_faucet: String,
    /// FEVM faucet address (f4/delegated)
    pub fevm_faucet: String,
    /// FOC deployer address (f4/delegated)
    pub foc_deployer: String,
    /// FOC deployer Ethereum address (0x)
    pub foc_deployer_eth: String,
    /// MockUSDFC token contract address
    pub mock_usdfc: String,
    /// Other deployed FOC contracts
    pub foc_contracts: std::collections::HashMap<String, String>,
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
    /// println!("MockUSDFC address: {}", addresses.mock_usdfc);
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
    ///
    /// let addresses = ContractAddresses {
    ///     global_fil_faucet: "f3abc...".to_string(),
    ///     // ... other fields
    ///     foc_contracts: std::collections::HashMap::new(),
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

    /// Check if all required addresses are present
    ///
    /// # Returns
    /// true if all required addresses are non-empty, false otherwise
    pub fn is_complete(&self) -> bool {
        !self.global_fil_faucet.is_empty()
            && !self.fevm_faucet.is_empty()
            && !self.foc_deployer.is_empty()
            && !self.foc_deployer_eth.is_empty()
            && !self.mock_usdfc.is_empty()
    }
}