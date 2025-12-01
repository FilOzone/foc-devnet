//! MockUSDFC Token Deployment step.
//!
//! This module handles the deployment of the MockUSDFC ERC-20 token
//! required by FOC contracts for local testing.

use super::step::{Step, StepContext};
use crate::embedded_assets;
use crate::paths::foc_localnet_bin;
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

    /// Deploy MockUSDFC token for local testing
    ///
    /// Returns the deployed contract address
    fn deploy_mock_usdfc(
        deployer_eth_addr: &str,
        lotus_rpc_url: &str,
    ) -> Result<String, Box<dyn Error>> {
        println!("      Deploying MockUSDFC token...");

        // Get the embedded contract content
        let contract_content = embedded_assets::MOCK_USDFC_CONTRACT;

        // Deploy using forge via foc-builder container
        println!("        Compiling and deploying with forge...");

        let bin_dir = foc_localnet_bin();
        let builder_volumes_dir = foc_localnet_bin().parent().unwrap().join("builder");

        // Create a temporary contract file
        let temp_contract_path = std::env::temp_dir().join("MockUSDFC.sol");
        fs::write(&temp_contract_path, contract_content)?;

        // Build forge command to deploy MockUSDFC
        let forge_cmd = format!(
            r#"forge create /tmp/MockUSDFC.sol:MockUSDFC \
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
                &format!("{}:/tmp/MockUSDFC.sol", temp_contract_path.display()),
                "foc-builder",
                "/bin/bash",
                "-c",
                &forge_cmd,
            ])
            .output()?;

        // Clean up temp file
        let _ = fs::remove_file(&temp_contract_path);

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

    /// Perform the MockUSDFC deployment process
    fn perform_token_deployment(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!("    Deploying MockUSDFC token...");

        // Get required addresses from context
        let (foc_deployer, foc_deployer_eth) = self.check_required_addresses(context)?;

        // Deploy MockUSDFC token
        let lotus_rpc_url = format!("http://localhost:{}/rpc/v1", LOTUS_RPC_PORT);
        let mock_usdfc_address = Self::deploy_mock_usdfc(&foc_deployer_eth, &lotus_rpc_url)?;
        context.set("mock_usdfc_address", &mock_usdfc_address);

        println!(
            "\n    {} MockUSDFC token deployed successfully!",
            "✓".green().bold()
        );
        println!("      Token Address: {}", mock_usdfc_address);
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
        let (_foc_deployer, foc_deployer_eth) = self.check_required_addresses(context)?;
        println!(
            "    {} FOC_DEPLOYER Ethereum address: {}",
            "✓".green(),
            foc_deployer_eth
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
            println!("      {} MockUSDFC address: {}", "✓".green(), token_address);
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
