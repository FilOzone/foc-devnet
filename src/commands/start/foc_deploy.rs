//! FOC (Filecoin Onchain Contracts) deployment step.
//!
//! This module handles deploying FOC contracts to the Lotus node with FEVM enabled.
//! These contracts are required by Curio for storage provider operations.

use super::step::{Step, StepContext};
use crate::paths::{
    contract_addresses_file, foc_localnet_bin, foc_localnet_docker_volumes,
    foc_localnet_filecoin_services_repo, foc_localnet_lotus_keys,
};
use crossterm::style::Stylize;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

// Service configuration
const SERVICE_NAME: &str = "FOC LocalNet Warm Storage";
const SERVICE_DESCRIPTION: &str = "Warm storage service for FOC local development network";

// Account configuration
const GLOBAL_FIL_FAUCET_KEY: &str = "prefunded-1"; // The GLOBAL_FIL_FAUCET account
const FEVM_FAUCET_AMOUNT: &str = "10000"; // 10,000 FIL to transfer to FEVM ecosystem
const FOC_DEPLOYER_AMOUNT: &str = "5000"; // 5,000 FIL for contract deployment

// Token configuration
const MOCK_USDFC_INITIAL_SUPPLY: &str = "1000000000000000000000000"; // 1 million tokens (18 decimals)

// Network configuration
const LOTUS_RPC_PORT: u16 = 1234;
const LOCAL_NETWORK_CHAIN_ID: u64 = 31415926; // Local network chain ID
const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 15;

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
    pub fn is_complete(&self) -> bool {
        !self.global_fil_faucet.is_empty()
            && !self.fevm_faucet.is_empty()
            && !self.foc_deployer.is_empty()
            && !self.foc_deployer_eth.is_empty()
            && !self.mock_usdfc.is_empty()
    }
}

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
            return Err("Lotus container is not running. FOC deployment requires Lotus to be running with FEVM enabled.".into());
        }

        Ok(())
    }
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

        // Extract the relative path from the lotus-keys directory
        // keyinfo_path is like: ~/.foc-localnet/artifacts/docker/volumes/lotus-keys/prefunded-1/bls-....keyinfo
        // We need to convert it to: /keys/prefunded-1/bls-....keyinfo
        let keys_dir = foc_localnet_lotus_keys();
        let relative_path = keyinfo_path
            .strip_prefix(&keys_dir)
            .map_err(|_| "Failed to get relative path for keyinfo")?;
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

        // Deploy using forge via foc-builder container
        println!("        Compiling and deploying with forge...");

        let bin_dir = foc_localnet_bin();
        let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");
        let contracts_dir = std::env::current_dir()?.join("contracts");

        // Build forge command to deploy MockUSDFC
        // Use the full path to the contract file with --contracts flag
        let forge_cmd = format!(
            r#"forge create /contracts/MockUSDFC.sol:MockUSDFC \
               --rpc-url {} \
               --from {} \
               --unlocked \
               --constructor-args {} \
               --json"#,
            lotus_rpc_url, deployer_eth_addr, MOCK_USDFC_INITIAL_SUPPLY
        );

        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--network",
                "host", // Use host network to access localhost:1234
                "-v",
                &format!("{}:/opt/bin", bin_dir.display()),
                "-v",
                &format!(
                    "{}:/home/foc-user/.cargo",
                    builder_volumes_dir.join("cargo").display()
                ),
                "-v",
                &format!("{}:/contracts", contracts_dir.display()),
                "foc-builder",
                "/bin/bash",
                "-c",
                &forge_cmd,
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("        {} Forge deployment failed", "✗".red());
            println!("        Error: {}", stderr);
            println!(
                "        {} Using deployer address as placeholder",
                "⚠".yellow()
            );
            return Ok(deployer_eth_addr.to_string());
        }

        // Parse the JSON output to get the deployed address
        let output_str = String::from_utf8_lossy(&output.stdout);

        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output_str) {
            if let Some(deployed_to) = json.get("deployedTo").and_then(|v| v.as_str()) {
                println!(
                    "        {} MockUSDFC deployed to: {}",
                    "✓".green(),
                    deployed_to
                );
                return Ok(deployed_to.to_string());
            }
        }

        // Fallback: try to find address in output
        for line in output_str.lines() {
            if line.contains("Deployed to:") {
                if let Some(addr) = line.split_whitespace().last() {
                    println!("        {} MockUSDFC deployed to: {}", "✓".green(), addr);
                    return Ok(addr.to_string());
                }
            }
        }

        println!(
            "        {} Could not parse deployment address",
            "⚠".yellow()
        );
        println!("        Using deployer address as placeholder");
        Ok(deployer_eth_addr.to_string())
    }

    /// Deploy FOC contracts using the deployment script
    ///
    /// Returns a map of contract names to addresses
    fn deploy_foc_contracts(
        deployer_eth_addr: &str,
        mock_usdfc_address: &str,
        lotus_rpc_url: &str,
    ) -> Result<std::collections::HashMap<String, String>, Box<dyn Error>> {
        println!("      Running deploy-all-warm-storage.sh...");

        let services_repo = foc_localnet_filecoin_services_repo();
        // Resolve symlinks to get the real path for Docker mounting
        let services_repo = services_repo
            .canonicalize()
            .unwrap_or_else(|_| services_repo.clone());
        let contracts_dir = services_repo.join("service_contracts");
        let deploy_script = contracts_dir
            .join("tools")
            .join("deploy-all-warm-storage.sh");

        if !deploy_script.exists() {
            return Err(
                format!("Deployment script not found at {}", deploy_script.display()).into(),
            );
        }

        let bin_dir = foc_localnet_bin();
        let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

        // Prepare environment variables for the deployment script
        let env_vars = format!(
            r#"export ETH_RPC_URL='{}'
export USDFC_TOKEN_ADDRESS='{}'
export SERVICE_NAME='{}'
export SERVICE_DESCRIPTION='{}'
export DRY_RUN=false
export CHAIN={}  # Local network chain ID
export DEPLOYER_ADDRESS='{}'
export AUTO_VERIFY=false"#,
            lotus_rpc_url,
            mock_usdfc_address,
            SERVICE_NAME,
            SERVICE_DESCRIPTION,
            LOCAL_NETWORK_CHAIN_ID,
            deployer_eth_addr
        );

        // Run the deployment script
        let deploy_cmd = format!(
            r#"{} && \
               cd /service_contracts && \
               bash /service_contracts/tools/deploy-all-warm-storage.sh 2>&1 | tee /tmp/foc-deploy.log"#,
            env_vars
        );

        println!("        This may take several minutes...");

        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--network",
                "host",
                "-v",
                &format!("{}:/opt/bin", bin_dir.display()),
                "-v",
                &format!(
                    "{}:/home/foc-user/.cargo",
                    builder_volumes_dir.join("cargo").display()
                ),
                "-v",
                &format!("{}:/service_contracts", services_repo.display()),
                "foc-builder",
                "/bin/bash",
                "-c",
                &deploy_cmd,
            ])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        // Print output for debugging
        println!("\n        Deployment output:");
        for line in output_str.lines() {
            println!("          {}", line);
        }

        if !output.status.success() {
            println!("        {} Deployment script failed", "✗".red());
            println!("        Error output:");
            for line in stderr_str.lines() {
                println!("          {}", line);
            }
            return Err("FOC contract deployment failed".into());
        }

        // Parse the deployment output to extract contract addresses
        let addresses = Self::parse_deployment_output(&output_str)?;

        Ok(addresses)
    }

    /// Parse deployment output to extract contract addresses
    fn parse_deployment_output(
        output_str: &str,
    ) -> Result<std::collections::HashMap<String, String>, Box<dyn Error>> {
        // Look for "DEPLOYMENT SUMMARY" section
        let mut addresses = std::collections::HashMap::new();

        // Look for "DEPLOYMENT SUMMARY" section
        let mut in_summary = false;
        for line in output_str.lines() {
            if line.contains("DEPLOYMENT SUMMARY") {
                in_summary = true;
                continue;
            }

            if in_summary && line.contains(":") && line.contains("0x") {
                // Parse lines like "PDPVerifier Implementation: 0x1234..."
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() == 2 {
                    let name = parts[0].trim();
                    let addr = parts[1].trim();
                    if addr.starts_with("0x") {
                        addresses.insert(name.to_string(), addr.to_string());
                    }
                }
            }

            // Stop parsing after configuration section
            if in_summary && line.contains("Network Configuration") {
                break;
            }
        }

        if addresses.is_empty() {
            println!(
                "        {} No contract addresses found in output",
                "⚠".yellow()
            );
            println!("        Deployment may have failed or output format changed");
        } else {
            println!(
                "        {} Successfully deployed {} contracts",
                "✓".green(),
                addresses.len()
            );
        }

        Ok(addresses)
    }

    /// Setup deployment prerequisites including addresses and token deployment
    fn setup_deployment_prerequisites(
        &self,
        context: &mut StepContext,
    ) -> Result<(String, String, String, String, String), Box<dyn Error>> {
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
        let lotus_rpc_url = format!("http://localhost:{}/rpc/v1", LOTUS_RPC_PORT);
        let mock_usdfc_address = Self::deploy_mock_usdfc(&deployer_eth_addr, &lotus_rpc_url)?;
        context.set("mock_usdfc_address", &mock_usdfc_address);
        println!(
            "      {} MockUSDFC token address: {}",
            "✓".green(),
            mock_usdfc_address
        );

        println!(
            "\n    {} FOC deployment prerequisites ready!",
            "✓".green().bold()
        );
        println!("      GLOBAL_FIL_FAUCET: {}", global_faucet);
        println!("      FEVM_FAUCET: {}", fevm_faucet);
        println!("      FOC_DEPLOYER: {}", foc_deployer);
        println!("      FOC_DEPLOYER (ETH): {}", deployer_eth_addr);
        println!("      MockUSDFC Token: {}", mock_usdfc_address);

        Ok((
            global_faucet,
            fevm_faucet,
            foc_deployer,
            deployer_eth_addr,
            mock_usdfc_address,
        ))
    }

    /// Check if contracts are already deployed and handle early return
    fn check_existing_deployment(&self, context: &mut StepContext) -> Result<bool, Box<dyn Error>> {
        if let Ok(existing_addresses) = ContractAddresses::load() {
            if existing_addresses.is_complete() {
                println!(
                    "    {} Contracts already deployed, skipping deployment...",
                    "✓".green()
                );
                println!(
                    "      GLOBAL_FIL_FAUCET: {}",
                    existing_addresses.global_fil_faucet
                );
                println!("      FEVM_FAUCET: {}", existing_addresses.fevm_faucet);
                println!("      FOC_DEPLOYER: {}", existing_addresses.foc_deployer);
                println!(
                    "      FOC_DEPLOYER (ETH): {}",
                    existing_addresses.foc_deployer_eth
                );
                println!("      MockUSDFC Token: {}", existing_addresses.mock_usdfc);

                // Store in context for other steps
                self.store_addresses_in_context(context, &existing_addresses);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Store contract addresses in the step context
    fn store_addresses_in_context(&self, context: &mut StepContext, addresses: &ContractAddresses) {
        context.set("global_faucet_address", &addresses.global_fil_faucet);
        context.set("fevm_faucet_address", &addresses.fevm_faucet);
        context.set("foc_deployer_address", &addresses.foc_deployer);
        context.set("foc_deployer_eth_address", &addresses.foc_deployer_eth);
        context.set("mock_usdfc_address", &addresses.mock_usdfc);

        for (name, addr) in &addresses.foc_contracts {
            context.set(&format!("foc_contract_{}", name.replace(' ', "_")), addr);
        }
    }

    /// Perform the full deployment process
    fn perform_deployment(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Setting up FOC deployment prerequisites...");

        let (global_faucet, fevm_faucet, foc_deployer, deployer_eth_addr, mock_usdfc_address) =
            self.setup_deployment_prerequisites(context)?;

        let lotus_rpc_url = format!("http://localhost:{}/rpc/v1", LOTUS_RPC_PORT);

        self.deploy_contracts_and_save(
            context,
            &deployer_eth_addr,
            &mock_usdfc_address,
            &lotus_rpc_url,
            &global_faucet,
            &fevm_faucet,
            &foc_deployer,
        )?;
        Ok(())
    }

    /// Deploy contracts and save the results
    fn deploy_contracts_and_save(
        &self,
        context: &mut StepContext,
        deployer_eth_addr: &str,
        mock_usdfc_address: &str,
        lotus_rpc_url: &str,
        global_faucet: &str,
        fevm_faucet: &str,
        foc_deployer: &str,
    ) -> Result<(), Box<dyn Error>> {
        // Deploy FOC contracts using deployment script
        println!("\n    Deploying FOC contracts...");
        println!("      (This may take several minutes)");

        let contract_addresses =
            Self::deploy_foc_contracts(deployer_eth_addr, mock_usdfc_address, lotus_rpc_url)?;

        // Store contract addresses in context
        for (name, addr) in &contract_addresses {
            context.set(&format!("foc_contract_{}", name.replace(' ', "_")), addr);
        }

        // Save all addresses to the state file
        let addresses_struct = ContractAddresses {
            global_fil_faucet: global_faucet.to_string(),
            fevm_faucet: fevm_faucet.to_string(),
            foc_deployer: foc_deployer.to_string(),
            foc_deployer_eth: deployer_eth_addr.to_string(),
            mock_usdfc: mock_usdfc_address.to_string(),
            foc_contracts: contract_addresses.clone(),
        };

        addresses_struct.save()?;
        println!(
            "      {} Contract addresses saved to {}",
            "✓".green(),
            contract_addresses_file().display()
        );

        println!(
            "\n    {} FOC contracts deployed successfully!",
            "✓".green().bold()
        );
        println!("      Deployed {} contracts", contract_addresses.len());

        Ok(())
    }
}

impl Step for FOCDeployStep {
    /// Get the name of this step
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
        println!("    {} filecoin-services repository found", "✓".green());

        // Check if deployment script exists
        let deploy_script = services_repo
            .join("service_contracts")
            .join("tools")
            .join("deploy-all-warm-storage.sh");

        if !deploy_script.exists() {
            return Err(
                format!("Deployment script not found at {}", deploy_script.display()).into(),
            );
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

    /// Execute the FOC deployment process
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        if self.check_existing_deployment(context)? {
            return Ok(());
        }

        self.perform_deployment(context)?;
        Ok(())
    }

    /// Perform post-execution verification for FOC deployment
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying FOC deployment...");

        // Check if contracts were deployed
        let mut contract_count = 0;
        for (key, _) in &context.state {
            if key.starts_with("foc_contract_") {
                contract_count += 1;
            }
        }

        if contract_count > 0 {
            println!(
                "      {} {} contracts verified in context",
                "✓".green(),
                contract_count
            );
        } else {
            println!("      {} No contracts found in context", "⚠".yellow());
        }

        println!(
            "\n    {} FOC deployment step completed!",
            "✓".green().bold()
        );
        println!("      All prerequisites and contracts are deployed.");

        Ok(())
    }
}
