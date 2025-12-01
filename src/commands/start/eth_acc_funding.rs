//! Ethereum Account Funding step.
//!
//! This module handles the creation and funding of Ethereum-compatible accounts
//! required for FOC contract deployment. It creates f4 (delegated) addresses
//! and funds them with FIL for FEVM operations.

use super::step::{Step, StepContext};
use crate::paths::foc_localnet_lotus_keys;
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

// Account configuration constants
const GLOBAL_FIL_FAUCET_KEY: &str = "prefunded-1"; // The GLOBAL_FIL_FAUCET account
const FEVM_FAUCET_AMOUNT: &str = "10000"; // 10,000 FIL to transfer to FEVM ecosystem
const FOC_DEPLOYER_AMOUNT: &str = "5000"; // 5,000 FIL for contract deployment

// Network configuration
const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 15;

/// Step for funding Ethereum accounts required for FOC deployment
pub struct ETHAccFundingStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl ETHAccFundingStep {
    /// Create a new ETHAccFundingStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }

    /// Check if Lotus is running and accessible
    fn check_lotus_running() -> Result<(), Box<dyn Error>> {
        let output = Command::new("docker")
            .args([
                "ps",
                "--filter",
                "name=^foc-lotus$",
                "--format",
                "{{.Names}}",
            ])
            .output()?;

        if !String::from_utf8_lossy(&output.stdout)
            .trim()
            .contains("foc-lotus")
        {
            return Err("Lotus container is not running. ETH account funding requires Lotus to be running with FEVM enabled.".into());
        }

        Ok(())
    }

    /// Get the global faucet address from the prefunded key
    fn get_global_faucet_address() -> Result<String, Box<dyn Error>> {
        let keys_dir = foc_localnet_lotus_keys();
        let faucet_key_dir = keys_dir.join(GLOBAL_FIL_FAUCET_KEY);

        if !faucet_key_dir.exists() {
            return Err(format!(
                "GLOBAL_FIL_FAUCET key directory not found at {}. \
                 Ensure genesis preparation has created this key.",
                faucet_key_dir.display()
            )
            .into());
        }

        // Find the keyinfo file
        let entries: Vec<_> = fs::read_dir(&faucet_key_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.starts_with("bls-") && s.ends_with(".keyinfo"))
                    .unwrap_or(false)
            })
            .collect();

        if entries.is_empty() {
            return Err(
                format!("No BLS keyinfo file found in {}", faucet_key_dir.display()).into(),
            );
        }

        // Extract address from filename: bls-<address>.keyinfo
        let filename = entries[0].file_name();
        let filename_str = filename.to_str().ok_or("Invalid filename encoding")?;

        let address = filename_str
            .strip_prefix("bls-")
            .and_then(|s| s.strip_suffix(".keyinfo"))
            .ok_or("Invalid keyinfo filename format")?;

        Ok(address.to_string())
    }

    /// Import the GLOBAL_FIL_FAUCET key into Lotus wallet
    fn import_faucet_key(keyinfo_path: &PathBuf) -> Result<String, Box<dyn Error>> {
        println!("      Importing GLOBAL_FIL_FAUCET key into Lotus wallet...");

        // Read the JSON content from the keyinfo file
        let json_content = fs::read_to_string(keyinfo_path)
            .map_err(|e| format!("Failed to read keyinfo file: {}", e))?;

        // Hex-encode the JSON content (lotus wallet import expects hex-encoded JSON)
        let hex_encoded = hex::encode(json_content);

        // Create a temporary file with the hex-encoded content in the same directory
        // so it's accessible via the mounted volume
        let temp_key_file = keyinfo_path.with_extension("keyinfo.hex");
        fs::write(&temp_key_file, &hex_encoded)
            .map_err(|e| format!("Failed to write hex key file: {}", e))?;

        // Get the container path for the temp file
        let keys_dir = foc_localnet_lotus_keys();
        let relative_path = temp_key_file
            .strip_prefix(&keys_dir)
            .map_err(|_| "Failed to get relative path for hex key file")?;
        let container_path = format!("/keys/{}", relative_path.display());

        let output = Command::new("docker")
            .args([
                "exec",
                "foc-lotus",
                "/usr/local/bin/lotus-bins/lotus",
                "wallet",
                "import",
                &container_path,
            ])
            .output()?;

        // Clean up the temp file
        let _ = fs::remove_file(&temp_key_file);

        if !output.status.success() {
            return Err(format!(
                "Failed to import GLOBAL_FIL_FAUCET key: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let address = String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| line.starts_with("imported key"))
            .and_then(|line| line.split_whitespace().nth(2))
            .ok_or("Failed to extract imported address")?
            .to_string();

        println!("      {} Key imported: {}", "✓".green(), address);
        Ok(address)
    }

    /// Create a new f4 (delegated/Ethereum) address for FEVM operations
    fn create_fevm_address(name: &str) -> Result<String, Box<dyn Error>> {
        println!("      Creating {} f4 address...", name);

        let output = Command::new("docker")
            .args([
                "exec",
                "foc-lotus",
                "/usr/local/bin/lotus-bins/lotus",
                "wallet",
                "new",
                "delegated",
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to create {} f4 address: {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let address = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!(
            "      {} {} address created: {}",
            "✓".green(),
            name,
            address
        );
        Ok(address)
    }

    /// Transfer FIL from one address to another
    fn transfer_fil(
        from: &str,
        to: &str,
        amount: &str,
        description: &str,
    ) -> Result<(), Box<dyn Error>> {
        println!("      Transferring {} FIL: {}...", amount, description);

        let output = Command::new("docker")
            .args([
                "exec",
                "foc-lotus",
                "/usr/local/bin/lotus-bins/lotus",
                "send",
                "--from",
                from,
                to,
                amount,
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to transfer FIL: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        println!("      {} Transfer successful", "✓".green());

        // Wait for transaction to be included in a block and address to be activated
        // F4 addresses need time to be activated on-chain
        println!("      Waiting for transaction confirmation and address activation...");
        thread::sleep(Duration::from_secs(TRANSACTION_CONFIRMATION_WAIT_SECS));

        Ok(())
    }

    /// Get the Ethereum address corresponding to an f4 address
    fn get_eth_address(f4_address: &str) -> Result<String, Box<dyn Error>> {
        let output = Command::new("docker")
            .args([
                "exec",
                "foc-lotus",
                "/usr/local/bin/lotus-bins/lotus",
                "evm",
                "stat",
                f4_address,
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to get Ethereum address: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let eth_addr = output_str
            .lines()
            .find(|line| line.contains("Eth address:"))
            .and_then(|line| line.split_whitespace().nth(2))
            .ok_or("Failed to extract Ethereum address")?
            .to_string();

        Ok(eth_addr)
    }

    /// Export private key for an f4 address to use with forge/cast
    fn export_private_key(f4_address: &str, output_file: &PathBuf) -> Result<(), Box<dyn Error>> {
        println!("      Exporting private key for contract deployment...");

        let output = Command::new("docker")
            .args([
                "exec",
                "foc-lotus",
                "/usr/local/bin/lotus-bins/lotus",
                "wallet",
                "export",
                f4_address,
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to export private key: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        // Write the keyinfo to a file
        fs::write(output_file, &output.stdout)?;
        println!("      {} Private key exported", "✓".green());

        Ok(())
    }

    /// Check if account funding has already been completed
    fn check_existing_funding(&self, context: &mut StepContext) -> Result<bool, Box<dyn Error>> {
        // Check if we have the required addresses in context
        let has_global_faucet = context.get("global_faucet_address").is_some();
        let has_fevm_faucet = context.get("fevm_faucet_address").is_some();
        let has_foc_deployer = context.get("foc_deployer_address").is_some();
        let has_eth_address = context.get("foc_deployer_eth_address").is_some();

        if has_global_faucet && has_fevm_faucet && has_foc_deployer && has_eth_address {
            println!(
                "    {} Account funding already completed, skipping...",
                "✓".green()
            );
            return Ok(true);
        }

        Ok(false)
    }

    /// Perform the account funding process
    fn perform_account_funding(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Setting up Ethereum accounts for FOC deployment...");

        // Step 1: Import GLOBAL_FIL_FAUCET key
        let keys_dir = foc_localnet_lotus_keys();
        let faucet_key_dir = keys_dir.join(GLOBAL_FIL_FAUCET_KEY);
        let keyinfo_files: Vec<_> = fs::read_dir(&faucet_key_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.starts_with("bls-") && s.ends_with(".keyinfo"))
                    .unwrap_or(false)
            })
            .collect();

        if keyinfo_files.is_empty() {
            return Err("No keyinfo file found for GLOBAL_FIL_FAUCET".into());
        }

        let keyinfo_path = keyinfo_files[0].path();
        let global_faucet = Self::import_faucet_key(&keyinfo_path)?;
        context.set("global_faucet_address", &global_faucet);

        // Step 2: Create FEVM_FAUCET address
        let fevm_faucet = Self::create_fevm_address("FEVM_FAUCET")?;
        context.set("fevm_faucet_address", &fevm_faucet);

        // Step 3: Transfer FIL from GLOBAL_FIL_FAUCET to FEVM_FAUCET
        Self::transfer_fil(
            &global_faucet,
            &fevm_faucet,
            FEVM_FAUCET_AMOUNT,
            "GLOBAL_FIL_FAUCET → FEVM_FAUCET",
        )?;

        // Step 4: Create FOC_DEPLOYER address
        let foc_deployer = Self::create_fevm_address("FOC_DEPLOYER")?;
        context.set("foc_deployer_address", &foc_deployer);

        // Step 5: Transfer FIL from FEVM_FAUCET to FOC_DEPLOYER
        Self::transfer_fil(
            &fevm_faucet,
            &foc_deployer,
            FOC_DEPLOYER_AMOUNT,
            "FEVM_FAUCET → FOC_DEPLOYER",
        )?;

        // Step 6: Get Ethereum address for FOC_DEPLOYER
        let deployer_eth_addr = Self::get_eth_address(&foc_deployer)?;
        println!(
            "      {} FOC_DEPLOYER Ethereum address: {}",
            "✓".green(),
            deployer_eth_addr
        );
        context.set("foc_deployer_eth_address", &deployer_eth_addr);

        // Step 7: Export private key for FOC_DEPLOYER
        let deployer_key_file = self.volumes_dir.join("foc-deployer.key");
        Self::export_private_key(&foc_deployer, &deployer_key_file)?;

        println!(
            "\n    {} Ethereum accounts funded successfully!",
            "✓".green().bold()
        );
        println!("      GLOBAL_FIL_FAUCET: {}", global_faucet);
        println!("      FEVM_FAUCET: {}", fevm_faucet);
        println!("      FOC_DEPLOYER: {}", foc_deployer);
        println!("      FOC_DEPLOYER (ETH): {}", deployer_eth_addr);

        Ok(())
    }
}

impl Step for ETHAccFundingStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Fund Ethereum Accounts"
    }

    fn pre_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        Self::check_lotus_running()?;
        println!("    {} Lotus is running", "✓".green());

        // Check if GLOBAL_FIL_FAUCET key exists
        let faucet_addr = Self::get_global_faucet_address()?;
        println!(
            "    {} GLOBAL_FIL_FAUCET address: {}",
            "✓".green(),
            faucet_addr
        );

        Ok(())
    }

    /// Execute the account funding process
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        if self.check_existing_funding(context)? {
            return Ok(());
        }

        self.perform_account_funding(context)?;
        Ok(())
    }

    /// Perform post-execution verification for account funding
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying account funding...");

        // Check if all required addresses are in context
        let required_keys = vec![
            "global_faucet_address",
            "fevm_faucet_address",
            "foc_deployer_address",
            "foc_deployer_eth_address",
        ];

        for key in required_keys {
            if let Some(value) = context.get(key) {
                println!("      {} {}: {}", "✓".green(), key, value);
            } else {
                println!("      {} {} not found in context", "✗".red(), key);
                return Err(format!("Missing required address: {}", key).into());
            }
        }

        println!(
            "\n    {} Account funding step completed!",
            "✓".green().bold()
        );
        println!("      All Ethereum accounts are funded and ready for contract deployment.");

        Ok(())
    }
}
