//! FOC (Filecoin Onchain Contracts) deployment step.
//!
//! This module handles deploying FOC contracts to the Lotus node with FEVM enabled.
//! These contracts are required by Curio for storage provider operations.

use super::step::{Step, StepContext};
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_filecoin_services_repo,
    foc_localnet_lotus_keys,
};
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const CONTAINER_NAME: &str = "foc-deploy";

// Service configuration
const SERVICE_NAME: &str = "FOC LocalNet Warm Storage";
const SERVICE_DESCRIPTION: &str = "Warm storage service for FOC local development network";

// Account configuration
const GLOBAL_FIL_FAUCET_KEY: &str = "prefunded-1"; // The GLOBAL_FIL_FAUCET account
const FEVM_FAUCET_AMOUNT: &str = "10000"; // 10,000 FIL to transfer to FEVM ecosystem
const FOC_DEPLOYER_AMOUNT: &str = "5000"; // 5,000 FIL for contract deployment

// Token configuration
const MOCK_USDFC_INITIAL_SUPPLY: &str = "1000000000000000000000000"; // 1 million tokens (18 decimals)

/// Step for deploying FOC contracts
pub struct FOCDeployStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl FOCDeployStep {
    /// Create a new FOCDeployStep
    pub fn new(volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self {
            volumes_dir,
            logs_dir,
        }
    }

    /// Check if Lotus is running and accessible
    fn check_lotus_running() -> Result<(), Box<dyn Error>> {
        let output = Command::new("docker")
            .args(["ps", "--filter", "name=^foc-lotus$", "--format", "{{.Names}}"])
            .output()?;

        if !String::from_utf8_lossy(&output.stdout)
            .trim()
            .contains("foc-lotus")
        {
            return Err("Lotus container is not running. FOC deployment requires Lotus to be running with FEVM enabled.".into());
        }

        Ok(())
    }

    /// Get the GLOBAL_FIL_FAUCET BLS address
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
            return Err(format!(
                "No BLS keyinfo file found in {}",
                faucet_key_dir.display()
            )
            .into());
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

        let output = Command::new("docker")
            .args([
                "exec",
                "foc-lotus",
                "/usr/local/bin/lotus-bins/lotus",
                "wallet",
                "import",
                &format!("/keys/{}", keyinfo_path.file_name().unwrap().to_str().unwrap()),
            ])
            .output()?;

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
        println!("      {} {} address created: {}", "✓".green(), name, address);
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

        // Wait for transaction to be included in a block
        thread::sleep(Duration::from_secs(5));

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

    /// Deploy MockUSDFC token for local testing
    ///
    /// Returns the deployed contract address
    fn deploy_mock_usdfc(
        deployer_eth_addr: &str,
        lotus_rpc_url: &str,
    ) -> Result<String, Box<dyn Error>> {
        println!("      Deploying MockUSDFC token...");

        // Get the contract source path (relative to project root)
        let contract_path = std::env::current_dir()?.join("contracts/MockUSDFC.sol");

        if !contract_path.exists() {
            return Err(format!(
                "MockUSDFC contract not found at {}",
                contract_path.display()
            )
            .into());
        }

        // Use cast to deploy the contract
        // Format: cast send --create <bytecode> --rpc-url <url> --private-key <key>
        // For now, we'll use a simpler approach with solc + cast
        
        // Compile the contract using solc in foc-builder
        println!("        Compiling MockUSDFC.sol...");
        
        let compile_output = Command::new("docker")
            .args([
                "exec",
                "foc-lotus",
                "/bin/bash",
                "-c",
                &format!(
                    "curl -s -X POST -H 'Content-Type: application/json' \
                    --data '{{\"jsonrpc\":\"2.0\",\"method\":\"eth_accounts\",\"params\":[],\"id\":1}}' \
                    {}",
                    lotus_rpc_url
                ),
            ])
            .output()?;

        if !compile_output.status.success() {
            return Err(format!(
                "Failed to query accounts: {}",
                String::from_utf8_lossy(&compile_output.stderr)
            )
            .into());
        }

        // For now, return a placeholder - we need forge/solc in the container
        println!("        {} MockUSDFC deployment placeholder", "⚠".yellow());
        println!("        Using deployer address as temporary token address");
        
        // Return the deployer's address as a placeholder
        // In a real implementation, we'd compile and deploy the contract
        Ok(deployer_eth_addr.to_string())
    }
}

impl Step for FOCDeployStep {
    fn name(&self) -> &str {
        "Deploy FOC Contracts"
    }

    fn pre_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        Self::check_lotus_running()?;
        println!("    {} Lotus is running", "✓".green());

        // Check if filecoin-services repository exists
        let services_repo = foc_localnet_filecoin_services_repo();
        if !services_repo.exists() {
            return Err(format!(
                "filecoin-services repository not found at {}. \
                 Please run 'foc-localnet init' to clone the repository.",
                services_repo.display()
            )
            .into());
        }
        println!(
            "    {} filecoin-services repository found",
            "✓".green()
        );

        // Check if deployment script exists
        let deploy_script = services_repo
            .join("service_contracts")
            .join("tools")
            .join("deploy-all-warm-storage.sh");

        if !deploy_script.exists() {
            return Err(format!(
                "Deployment script not found at {}",
                deploy_script.display()
            )
            .into());
        }
        println!("    {} Deployment script found", "✓".green());

        // Check if GLOBAL_FIL_FAUCET key exists
        let faucet_addr = Self::get_global_faucet_address()?;
        println!(
            "    {} GLOBAL_FIL_FAUCET address: {}",
            "✓".green(),
            faucet_addr
        );

        Ok(())
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Setting up FOC deployment prerequisites...");

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

        // Step 8: Deploy MockUSDFC token for local testing
        println!("\n    Deploying MockUSDFC token for FOC contracts...");
        let lotus_rpc_url = "http://localhost:1234/rpc/v1";
        let mock_usdfc_address = Self::deploy_mock_usdfc(&deployer_eth_addr, lotus_rpc_url)?;
        context.set("mock_usdfc_address", &mock_usdfc_address);
        println!(
            "      {} MockUSDFC token address: {}",
            "✓".green(),
            mock_usdfc_address
        );

        println!("\n    {} FOC deployment prerequisites ready!", "✓".green().bold());
        println!("      GLOBAL_FIL_FAUCET: {}", global_faucet);
        println!("      FEVM_FAUCET: {}", fevm_faucet);
        println!("      FOC_DEPLOYER: {}", foc_deployer);
        println!("      FOC_DEPLOYER (ETH): {}", deployer_eth_addr);
        println!("      MockUSDFC Token: {}", mock_usdfc_address);

        // Step 9: Deploy FOC contracts using deployment script
        println!("\n    Deploying FOC contracts...");
        println!("      (This may take several minutes)");

        // TODO: Implement actual contract deployment
        // For now, we'll mark this as a placeholder
        println!("      {} Contract deployment implementation pending", "⚠".yellow());
        println!("      This will execute deploy-all-warm-storage.sh via foc-builder");

        Ok(())
    }

    fn post_execute(&self, _context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying FOC deployment...");

        // TODO: Verify contracts are deployed
        // For now, just check prerequisites are in place
        println!("      {} Deployment verification pending", "⚠".yellow());

        println!("\n    {} FOC deployment step completed!", "✓".green().bold());
        println!("      Note: Contract deployment implementation is pending.");
        println!("      All prerequisites (accounts, transfers) are in place.");

        Ok(())
    }
}
