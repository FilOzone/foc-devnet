//! FOC (Filecoin Onchain Contracts) deployment step.
//!
//! This module handles deploying FOC service contracts to the Lotus node with FEVM enabled.
//! These contracts are required by Curio for storage provider operations.

use super::step::{Step, StepContext};
use crate::config::{Config, Location};
use crate::paths::{
    contract_addresses_file, foc_localnet_bin, foc_localnet_config, foc_localnet_docker_volumes,
    foc_localnet_filecoin_services_repo,
};
use crossterm::style::Stylize;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// Service configuration
const SERVICE_NAME: &str = "FOC LocalNet Warm Storage";
const SERVICE_DESCRIPTION: &str = "Warm storage service for FOC local development network";

// Network configuration
const LOTUS_RPC_PORT: u16 = 1234;
const LOCAL_NETWORK_CHAIN_ID: u64 = 31415926; // Local network chain ID

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

/// Step for deploying FOC service contracts
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

    /// Get the filecoin-services repository path based on configuration
    fn get_filecoin_services_repo_path() -> Result<PathBuf, Box<dyn Error>> {
        // Load configuration
        let config_path = foc_localnet_config();
        let config_content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file at {:?}: {}", config_path, e))?;
        let config: Config = toml::from_str(&config_content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        // Determine the repository path based on location
        let repo_path = match &config.filecoin_services {
            Location::LocalSource { dir } => {
                // For LocalSource, use the configured directory directly
                PathBuf::from(dir)
            }
            _ => {
                // For Git-based locations, use the foc-localnet directory
                foc_localnet_filecoin_services_repo()
            }
        };

        Ok(repo_path)
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

    /// Check if required addresses are available in context
    fn check_required_addresses(&self, context: &StepContext) -> Result<(String, String, String, String), Box<dyn Error>> {
        let foc_deployer = context
            .get("foc_deployer_address")
            .ok_or("FOC_DEPLOYER address not found in context. Ensure ETHAccFunding step has been completed.")?;

        let foc_deployer_eth = context
            .get("foc_deployer_eth_address")
            .ok_or("FOC_DEPLOYER Ethereum address not found in context. Ensure ETHAccFunding step has been completed.")?;

        let mock_usdfc = context
            .get("mock_usdfc_address")
            .ok_or("MockUSDFC address not found in context. Ensure USDFCDeploy step has been completed.")?;

        let global_faucet = context
            .get("global_faucet_address")
            .ok_or("GLOBAL_FIL_FAUCET address not found in context. Ensure ETHAccFunding step has been completed.")?;

        Ok((foc_deployer.clone(), foc_deployer_eth.clone(), mock_usdfc.clone(), global_faucet.clone()))
    }

    /// Get the private key for an f4 address in hex format (for use with cast/forge)
    fn get_private_key(f4_address: &str) -> Result<String, Box<dyn Error>> {
        // Export the private key from lotus
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

        // The output is hex-encoded JSON
        let hex_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Decode from hex to get the JSON string
        let json_bytes =
            hex::decode(&hex_str).map_err(|e| format!("Failed to decode hex output: {}", e))?;

        let keyinfo_str = String::from_utf8(json_bytes)
            .map_err(|e| format!("Failed to convert bytes to string: {}", e))?;

        // Parse the JSON to extract the private key
        let keyinfo: serde_json::Value = serde_json::from_str(&keyinfo_str)
            .map_err(|e| format!("Failed to parse keyinfo JSON: {}", e))?;

        // The private key is in the "PrivateKey" field as a base64 string
        let private_key_b64 = keyinfo
            .get("PrivateKey")
            .and_then(|v| v.as_str())
            .ok_or("PrivateKey field not found in keyinfo")?;

        // Decode from base64
        use base64::{engine::general_purpose, Engine as _};
        let private_key_bytes = general_purpose::STANDARD
            .decode(private_key_b64)
            .map_err(|e| format!("Failed to decode private key from base64: {}", e))?;

        // Convert to hex string with 0x prefix
        let private_key_hex = format!("0x{}", hex::encode(&private_key_bytes));

        Ok(private_key_hex)
    }

    /// Deploy FOC contracts using the deployment script
    ///
    /// Returns a map of contract names to addresses
    fn deploy_foc_contracts(
        foc_deployer: &str,
        deployer_eth_addr: &str,
        mock_usdfc_address: &str,
        lotus_rpc_url: &str,
    ) -> Result<std::collections::HashMap<String, String>, Box<dyn Error>> {
        println!("      Running deploy-all-warm-storage.sh...");

        let services_repo = Self::get_filecoin_services_repo_path()?;
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

        // Get the private key from lotus for the deployer address
        let private_key = Self::get_private_key(foc_deployer)?;

        // Prepare environment variables for the deployment script
        let env_vars = format!(
            r#"export ETH_RPC_URL='{}'
export USDFC_TOKEN_ADDRESS='{}'
export SERVICE_NAME='{}'
export SERVICE_DESCRIPTION='{}'
export DRY_RUN=false
export CHAIN={}
export DEPLOYER_ADDRESS='{}'
export AUTO_VERIFY=false
export ETH_PRIVATE_KEY='{}'
export PASSWORD=''"#,
            lotus_rpc_url,
            mock_usdfc_address,
            SERVICE_NAME,
            SERVICE_DESCRIPTION,
            LOCAL_NETWORK_CHAIN_ID,
            deployer_eth_addr,
            private_key
        );

        // Run the deployment script
        // First, create a keystore from the private key with empty password
        let deploy_cmd = format!(
            r#"set -e
cast wallet import foc-deployer --private-key {} --unsafe-password ''
export ETH_KEYSTORE="$HOME/.foundry/keystores/foc-deployer"
{}
cd /service_contracts
bash /service_contracts/tools/deploy-all-warm-storage.sh 2>&1 | tee /tmp/foc-deploy.log"#,
            private_key, env_vars
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
                &format!("{}:/service_contracts", contracts_dir.display()),
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

    /// Check if FOC contracts are already deployed
    fn check_existing_deployment(&self, context: &mut StepContext) -> Result<bool, Box<dyn Error>> {
        if let Ok(existing_addresses) = ContractAddresses::load() {
            if !existing_addresses.foc_contracts.is_empty() {
                println!(
                    "    {} FOC contracts already deployed, skipping deployment...",
                    "✓".green()
                );

                // Store contract addresses in context
                for (name, addr) in &existing_addresses.foc_contracts {
                    context.set(&format!("foc_contract_{}", name.replace(' ', "_")), addr);
                }
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Perform the FOC contract deployment process
    fn perform_deployment(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Deploying FOC service contracts...");

        // Get required addresses from context
        let (foc_deployer, foc_deployer_eth, mock_usdfc_address, global_faucet) = self.check_required_addresses(context)?;

        let lotus_rpc_url = format!("http://localhost:{}/rpc/v1", LOTUS_RPC_PORT);

        // Deploy FOC contracts using deployment script
        let contract_addresses = Self::deploy_foc_contracts(
            &foc_deployer,
            &foc_deployer_eth,
            &mock_usdfc_address,
            &lotus_rpc_url,
        )?;

        // Store contract addresses in context
        for (name, addr) in &contract_addresses {
            context.set(&format!("foc_contract_{}", name.replace(' ', "_")), addr);
        }

        // Load existing addresses and update with FOC contracts
        let mut addresses_struct = ContractAddresses::load().unwrap_or_else(|_| ContractAddresses {
            global_fil_faucet: global_faucet,
            fevm_faucet: context.get("fevm_faucet_address").unwrap_or(&String::new()).clone(),
            foc_deployer: foc_deployer,
            foc_deployer_eth: foc_deployer_eth,
            mock_usdfc: mock_usdfc_address,
            foc_contracts: std::collections::HashMap::new(),
        });

        addresses_struct.foc_contracts = contract_addresses.clone();

        addresses_struct.save()?;
        println!(
            "      {} Contract addresses saved to {}",
            "✓".green(),
            contract_addresses_file().display()
        );

        println!(
            "\n    {} FOC service contracts deployed successfully!",
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

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        Self::check_lotus_running()?;
        println!("    {} Lotus is running", "✓".green());

        // Check if filecoin-services repository exists
        let services_repo = Self::get_filecoin_services_repo_path()?;
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

        // Check if required addresses are available
        let (_foc_deployer, foc_deployer_eth, mock_usdfc, _global_faucet) = self.check_required_addresses(context)?;
        println!(
            "    {} FOC_DEPLOYER Ethereum address: {}",
            "✓".green(),
            foc_deployer_eth
        );
        println!(
            "    {} MockUSDFC token address: {}",
            "✓".green(),
            mock_usdfc
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
        println!("      All FOC service contracts are deployed and ready.");

        Ok(())
    }
}