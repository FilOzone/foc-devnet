//! MockUSDFC Token Deployment step.
//!
//! This module handles the deployment of the MockUSDFC ERC-20 token
//! using the Foundry project located in contracts/MockUSDFC/.
//!
//! The deployment is delegated to the Foundry scripts which handle:
//! - Contract compilation with OpenZeppelin dependencies
//! - Deployment via forge script
//! - Verification of deployed contract functions
//!
//! This approach provides better separation of concerns and easier debugging.

use super::step::{Step, StepContext};
use crate::paths::{contract_addresses_file, project_root};
use crossterm::style::Stylize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// Token configuration
const MOCK_USDFC_INITIAL_SUPPLY: &str = "1000000000000000000000000"; // 1 million tokens (18 decimals)

// Network configuration
const LOTUS_RPC_PORT: u16 = 1234;

/// Step for deploying MockUSDFC token
pub struct USDFCDeployStep {
    volumes_dir: PathBuf,
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl USDFCDeployStep {
    /// Create a new USDFCDeployStep
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
            return Err("Lotus container is not running. MockUSDFC deployment requires Lotus to be running with FEVM enabled.".into());
        }

        Ok(())
    }

    /// Get the private key for the deployer from the exported key file
    fn get_deployer_private_key(
        &self,
        _foc_deployer_address: &str,
    ) -> Result<String, Box<dyn Error>> {
        use base64::{engine::general_purpose, Engine as _};

        // The key was exported by ETHAccFundingStep to volumes_dir/foc-deployer.key
        let key_file = self.volumes_dir.join("foc-deployer.key");

        if !key_file.exists() {
            return Err(format!(
                "Deployer key file not found at {}. \
                 Ensure ETHAccFunding step has completed successfully.",
                key_file.display()
            )
            .into());
        }

        // Read the hex-encoded JSON from the file
        let hex_str = fs::read_to_string(&key_file)?.trim().to_string();

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
        let private_key_bytes = general_purpose::STANDARD
            .decode(private_key_b64)
            .map_err(|e| format!("Failed to decode private key from base64: {}", e))?;

        // Convert to hex string with 0x prefix
        let private_key_hex = format!("0x{}", hex::encode(&private_key_bytes));

        Ok(private_key_hex)
    }

    /// Check if required addresses are available in context
    fn check_required_addresses(
        &self,
        context: &StepContext,
    ) -> Result<(String, String), Box<dyn Error>> {
        let foc_deployer = context
            .get("foc_deployer_address")
            .ok_or("FOC_DEPLOYER address not found in context. Ensure ETHAccFunding step has been completed.")?;

        let foc_deployer_eth = context
            .get("foc_deployer_eth_address")
            .ok_or("FOC_DEPLOYER Ethereum address not found in context. Ensure ETHAccFunding step has been completed.")?;

        Ok((foc_deployer.clone(), foc_deployer_eth.clone()))
    }

    /// Check if MockUSDFC has already been deployed
    fn check_existing_deployment(&self, context: &StepContext) -> bool {
        context.get("mock_usdfc_address").is_some()
    }

    /// Setup the Foundry project (install dependencies if needed)
    fn setup_foundry_project(contract_dir: &PathBuf) -> Result<(), Box<dyn Error>> {
        let openzeppelin_path = contract_dir.join("lib/openzeppelin-contracts");

        if !openzeppelin_path.exists() {
            println!("      Installing OpenZeppelin contracts...");

            // First, initialize git repo if it doesn't exist
            let git_dir = contract_dir.join(".git");
            if !git_dir.exists() {
                println!("        Initializing git repository...");
                let output = Command::new("docker")
                    .args([
                        "run",
                        "--rm",
                        "-v",
                        &format!("{}:/workspace", contract_dir.display()),
                        "foc-builder",
                        "bash",
                        "-c",
                        "cd /workspace && git init && git config user.email 'foc@localnet' && git config user.name 'FOC Localnet'",
                    ])
                    .output()?;

                if !output.status.success() {
                    return Err(format!(
                        "Failed to initialize git repository: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )
                    .into());
                }
            }

            // Install dependencies
            let output = Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "-v",
                    &format!("{}:/workspace", contract_dir.display()),
                    "foc-builder",
                    "bash",
                    "-c",
                    "cd /workspace && \
                     forge install OpenZeppelin/openzeppelin-contracts@v5.0.0 && \
                     forge install foundry-rs/forge-std",
                ])
                .output()?;

            if !output.status.success() {
                return Err(format!(
                    "Failed to install dependencies: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }

            println!("        {} Dependencies installed", "✓".green());
        }

        // Build contracts
        println!("      Building MockUSDFC contract...");
        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-v",
                &format!("{}:/workspace", contract_dir.display()),
                "foc-builder",
                "bash",
                "-c",
                "cd /workspace && forge build",
            ])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "Failed to build contracts: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        println!("        {} Contracts built", "✓".green());
        Ok(())
    }

    /// Deploy MockUSDFC using the Foundry project
    fn deploy_mock_usdfc_foundry(
        private_key: &str,
        lotus_rpc_url: &str,
    ) -> Result<String, Box<dyn Error>> {
        println!("      Deploying MockUSDFC using Foundry project...");

        // Get the contract directory
        let project_root = project_root()?;
        let contract_dir = project_root.join("contracts/MockUSDFC");

        if !contract_dir.exists() {
            return Err(format!(
                "MockUSDFC Foundry project not found at: {}",
                contract_dir.display()
            )
            .into());
        }

        // Setup the Foundry project (install deps, build)
        Self::setup_foundry_project(&contract_dir)?;

        // Deploy using forge script with explicit gas limit for FEVM
        println!("      Executing deployment script...");

        let deploy_cmd = format!(
            "cd /workspace && \
             forge script script/Deploy.s.sol:DeployMockUSDFC \
             --rpc-url {} \
             --private-key {} \
             --broadcast \
             --slow \
             --gas-estimate-multiplier 10000 \
             -vv",
            lotus_rpc_url, private_key
        );

        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--network",
                "host", // Use host network to access localhost:1234
                "-v",
                &format!("{}:/workspace", contract_dir.display()),
                "foc-builder",
                "bash",
                "-c",
                &deploy_cmd,
            ])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Print output for debugging
        if !stdout.is_empty() {
            println!("        Deployment output:");
            for line in stdout.lines() {
                println!("          {}", line);
            }
        }

        if !output.status.success() {
            println!("        {} Deployment failed", "✗".red());
            if !stderr.is_empty() {
                println!("        Error output:");
                for line in stderr.lines() {
                    println!("          {}", line);
                }
            }
            return Err("MockUSDFC deployment failed".into());
        }

        // Extract contract address from output
        // Look for "MockUSDFC deployed at:" in the output
        let contract_address = stdout
            .lines()
            .find(|line| line.contains("MockUSDFC deployed at:"))
            .and_then(|line| line.split_whitespace().last())
            .ok_or("Failed to extract contract address from deployment output")?;

        println!(
            "        {} MockUSDFC deployed at: {}",
            "✓".green(),
            contract_address.cyan().bold()
        );

        Ok(contract_address.to_string())
    }

    /// Verify the deployed MockUSDFC contract
    fn verify_mock_usdfc(
        private_key: &str,
        contract_address: &str,
        lotus_rpc_url: &str,
    ) -> Result<(), Box<dyn Error>> {
        println!("      Verifying MockUSDFC contract functions...");

        let project_root = project_root()?;
        let contract_dir = project_root.join("contracts/MockUSDFC");

        // Wait a bit for transaction confirmation
        println!("        Waiting for transaction confirmation...");
        std::thread::sleep(std::time::Duration::from_secs(6));

        let verify_cmd = format!(
            "cd /workspace && \
             forge script script/Verify.s.sol:VerifyMockUSDFC \
             --rpc-url {} \
             --private-key {} \
             --sig 'run(address)' {} \
             -vv",
            lotus_rpc_url, private_key, contract_address
        );

        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--network",
                "host",
                "-v",
                &format!("{}:/workspace", contract_dir.display()),
                "foc-builder",
                "bash",
                "-c",
                &verify_cmd,
            ])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Print verification output
        if !stdout.is_empty() {
            println!("        Verification output:");
            for line in stdout.lines() {
                println!("          {}", line);
            }
        }

        if !output.status.success() {
            println!("        {} Verification failed", "⚠".yellow());
            if !stderr.is_empty() {
                println!("        Error output:");
                for line in stderr.lines() {
                    println!("          {}", line);
                }
            }
            // Don't fail the step, just warn
            println!(
                "        {} Continuing despite verification warning",
                "→".cyan()
            );
        } else {
            println!("        {} All contract functions verified", "✓".green());
        }

        Ok(())
    }

    /// Save contract address to the contract addresses file
    fn save_contract_address(name: &str, address: &str) -> Result<(), Box<dyn Error>> {
        let file_path = contract_addresses_file();

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read existing addresses or create new file
        let mut addresses: serde_json::Value = if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        // Add/update the address
        addresses[name] = serde_json::json!(address);

        // Write back to file
        let content = serde_json::to_string_pretty(&addresses)?;
        fs::write(&file_path, content)?;

        println!(
            "        {} Contract address saved to {}",
            "✓".green(),
            file_path.display()
        );

        Ok(())
    }

    /// Perform the MockUSDFC deployment process
    fn perform_token_deployment(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Deploying MockUSDFC token using Foundry project...");

        // Get required addresses from context
        let (foc_deployer, foc_deployer_eth) = self.check_required_addresses(context)?;

        // Get deployer private key from the exported key file
        let private_key = self.get_deployer_private_key(&foc_deployer)?;

        println!("      Deployer ETH address: {}", foc_deployer_eth.cyan());

        // Deploy MockUSDFC token using Foundry
        let lotus_rpc_url = format!("http://localhost:{}/rpc/v1", LOTUS_RPC_PORT);
        let mock_usdfc_address = Self::deploy_mock_usdfc_foundry(&private_key, &lotus_rpc_url)?;

        // Store in context
        context.set("mock_usdfc_address", &mock_usdfc_address);

        // Save to contract addresses file
        Self::save_contract_address("MockUSDFC", &mock_usdfc_address)?;

        // Verify the deployment
        Self::verify_mock_usdfc(&private_key, &mock_usdfc_address, &lotus_rpc_url)?;

        println!(
            "\n    {} MockUSDFC token deployed successfully!",
            "✓".green().bold()
        );
        println!("      Token Address: {}", mock_usdfc_address.cyan().bold());
        println!("      Initial Supply: {} tokens", MOCK_USDFC_INITIAL_SUPPLY);
        println!("      Decimals: 18");

        Ok(())
    }
}

impl Step for USDFCDeployStep {
    /// Get the name of this step
    fn name(&self) -> &str {
        "Deploy MockUSDFC Token"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        // Check if Lotus is running
        Self::check_lotus_running()?;
        println!("    {} Lotus is running", "✓".green());

        // Check if required addresses are available
        let (foc_deployer, foc_deployer_eth) = self.check_required_addresses(context)?;
        println!(
            "    {} FOC_DEPLOYER address: {}",
            "✓".green(),
            foc_deployer.cyan()
        );
        println!(
            "    {} FOC_DEPLOYER Ethereum address: {}",
            "✓".green(),
            foc_deployer_eth.cyan()
        );

        Ok(())
    }

    /// Execute the token deployment process
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        if self.check_existing_deployment(context) {
            println!(
                "    {} MockUSDFC token already deployed, skipping...",
                "✓".green()
            );
            return Ok(());
        }

        self.perform_token_deployment(context)?;
        Ok(())
    }

    /// Perform post-execution verification for token deployment
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Verifying MockUSDFC deployment...");

        // Check if token address is in context
        if let Some(token_address) = context.get("mock_usdfc_address") {
            println!(
                "      {} MockUSDFC address: {}",
                "✓".green(),
                token_address.as_str().cyan().bold()
            );
        } else {
            println!("      {} MockUSDFC address not found in context", "✗".red());
            return Err("MockUSDFC deployment failed - no address in context".into());
        }

        println!(
            "\n    {} MockUSDFC deployment step completed!",
            "✓".green().bold()
        );
        println!("      Token is ready for FOC contract deployment.");

        Ok(())
    }
}
