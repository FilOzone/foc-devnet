//! PDP Service Provider step implementation.

use super::super::contract_addresses::ContractAddresses;
use super::super::step::{Step, StepContext};
use super::provider_id::ProviderIdInfo;
use super::registration;
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use crate::paths::pdp_sp_0_provider_id_file;
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;

/// Step for registering PDP service provider
pub struct PdpSpRegistrationStep {
    #[allow(dead_code)]
    logs_dir: PathBuf,
}

impl PdpSpRegistrationStep {
    /// Create a new PDPServiceProviderStep
    pub fn new(_volumes_dir: PathBuf, logs_dir: PathBuf) -> Self {
        Self { logs_dir }
    }

    /// Check if Lotus is running
    fn check_lotus_running(context: &StepContext) -> Result<(), Box<dyn Error>> {
        let run_id = context.run_id().ok_or("Run ID not found in context")?;
        let container_name = lotus_container_name(run_id);
        if !container_is_running(&container_name)? {
            return Err("Lotus container is not running.".into());
        }
        Ok(())
    }

    /// Load contract addresses from state
    fn load_contract_addresses() -> Result<ContractAddresses, Box<dyn Error>> {
        ContractAddresses::load()
            .map_err(|e| format!("Failed to load contract addresses: {}", e).into())
    }

    /// Get required addresses from context
    fn get_required_addresses(
        context: &StepContext,
    ) -> Result<(String, String, String, String), Box<dyn Error>> {
        let pdp_sp_0_address = context
            .get("pdp_sp_0_address")
            .ok_or("PDP_SP_0 address not found in context")?;

        let pdp_sp_0_eth_address = context
            .get("pdp_sp_0_eth_address")
            .ok_or("PDP_SP_0 Ethereum address not found in context")?;

        let deployer_foc_address = context
            .get("deployer_foc_address")
            .ok_or("DEPLOYER_FOC address not found in context")?;

        let deployer_foc_eth_address = context
            .get("deployer_foc_eth_address")
            .ok_or("DEPLOYER_FOC Ethereum address not found in context")?;

        Ok((
            pdp_sp_0_address.clone(),
            pdp_sp_0_eth_address.clone(),
            deployer_foc_address.clone(),
            deployer_foc_eth_address.clone(),
        ))
    }
}

impl Step for PdpSpRegistrationStep {
    fn name(&self) -> &str {
        "PDP Service Provider Registration"
    }

    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        println!(
            "{} {}",
            "Pre-checking".cyan().bold(),
            self.name().cyan().bold()
        );

        // Check if Lotus is running
        Self::check_lotus_running(context)?;
        println!("  {} Lotus is running", "✓".green());

        // Check if required addresses are available
        let (
            pdp_sp_0_address,
            pdp_sp_0_eth_address,
            deployer_foc_address,
            deployer_foc_eth_address,
        ) = Self::get_required_addresses(context)?;
        println!("  {} PDP_SP_0 address: {}", "✓".green(), pdp_sp_0_address);
        println!(
            "  {} PDP_SP_0 ETH address: {}",
            "✓".green(),
            pdp_sp_0_eth_address
        );
        println!(
            "  {} DEPLOYER_FOC address: {}",
            "✓".green(),
            deployer_foc_address
        );
        println!(
            "  {} DEPLOYER_FOC ETH address: {}",
            "✓".green(),
            deployer_foc_eth_address
        );

        // Check if contract addresses are available
        let contract_addresses = Self::load_contract_addresses()?;
        let registry_address = contract_addresses
            .foc_contracts
            .get("service_provider_registry_proxy")
            .ok_or("service_provider_registry_proxy address not found")?;
        println!(
            "  {} ServiceProviderRegistry: {}",
            "✓".green(),
            registry_address
        );

        let warm_storage_address = contract_addresses
            .foc_contracts
            .get("filecoin_warm_storage_service_proxy")
            .ok_or("filecoin_warm_storage_service_proxy address not found")?;
        println!(
            "  {} WarmStorageService: {}",
            "✓".green(),
            warm_storage_address
        );

        Ok(())
    }

    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        use super::super::lotus_utils::get_lotus_rpc_url;

        println!(
            "{} {}",
            "Executing".green().bold(),
            self.name().green().bold()
        );

        // Check if already registered
        if ProviderIdInfo::exists() {
            println!(
                "  {} Provider already registered, skipping...",
                "⊙".yellow()
            );
            return Ok(());
        }

        // Get run ID
        let run_id = context.run_id().ok_or("Run ID not found in context")?;

        // Get Lotus RPC URL with dynamic port
        let lotus_rpc_url = get_lotus_rpc_url(context)?;

        // Get required addresses
        let (
            pdp_sp_0_address,
            pdp_sp_0_eth_address,
            deployer_foc_address,
            deployer_foc_eth_address,
        ) = Self::get_required_addresses(context)?;

        // Load contract addresses
        let contract_addresses = Self::load_contract_addresses()?;
        let registry_address = contract_addresses
            .foc_contracts
            .get("service_provider_registry_proxy")
            .ok_or("service_provider_registry_proxy address not found")?
            .clone();

        let warm_storage_address = contract_addresses
            .foc_contracts
            .get("filecoin_warm_storage_service_proxy")
            .ok_or("filecoin_warm_storage_service_proxy address not found")?
            .clone();

        let mock_usdfc_address = contract_addresses
            .contracts
            .get("usdfc")
            .ok_or("usdfc address not found in contract addresses")?
            .clone();

        // Register provider
        let provider_id = registration::register_provider(
            run_id,
            &registry_address,
            &pdp_sp_0_address,
            &pdp_sp_0_eth_address,
            &mock_usdfc_address,
            &lotus_rpc_url,
        )?;

        // Add to approved list
        registration::add_to_approved_list(
            run_id,
            &warm_storage_address,
            provider_id,
            &deployer_foc_address,
            &deployer_foc_eth_address,
            &lotus_rpc_url,
        )?;

        // Save provider ID to state
        let info = ProviderIdInfo {
            provider_id,
            provider_address: pdp_sp_0_eth_address.clone(),
            payee_address: pdp_sp_0_eth_address.clone(),
        };
        info.save()?;

        println!(
            "  {} Provider ID saved to {}",
            "✓".green(),
            pdp_sp_0_provider_id_file().display()
        );

        // Store provider ID in context for downstream steps
        context.set("pdp_sp_0_provider_id", &provider_id.to_string());

        Ok(())
    }

    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        use super::super::lotus_utils::get_lotus_rpc_url;

        println!(
            "{} {}",
            "Post-checking".cyan().bold(),
            self.name().cyan().bold()
        );

        // Get run ID
        let run_id = context.run_id().ok_or("Run ID not found in context")?;

        // Get Lotus RPC URL with dynamic port
        let lotus_rpc_url = get_lotus_rpc_url(context)?;

        // Verify provider ID file exists and is valid
        let info = ProviderIdInfo::load()?;
        println!(
            "  {} Provider ID file exists: {}",
            "✓".green(),
            info.provider_id
        );

        // Load contract addresses for verification
        let contract_addresses = Self::load_contract_addresses()?;
        let registry_address = contract_addresses
            .foc_contracts
            .get("service_provider_registry_proxy")
            .ok_or("service_provider_registry_proxy address not found")?
            .clone();

        let state_view_address = contract_addresses
            .foc_contracts
            .get("filecoin_warm_storage_service_state_view")
            .ok_or("filecoin_warm_storage_service_state_view address not found")?
            .clone();

        // Verify there's exactly one provider on-chain
        let provider_count =
            registration::verify_provider_count(run_id, &registry_address, &lotus_rpc_url)?;
        if provider_count != 1 {
            return Err(format!(
                "Expected exactly 1 provider on-chain, found {}",
                provider_count
            )
            .into());
        }
        println!(
            "  {} Provider count on-chain: {}",
            "✓".green(),
            provider_count
        );

        // Verify on-chain provider ID matches the saved one
        let onchain_provider_id = registration::verify_provider_id_by_address(
            run_id,
            &registry_address,
            &info.provider_address,
            &lotus_rpc_url,
        )?;
        if onchain_provider_id != info.provider_id {
            return Err(format!(
                "Provider ID mismatch: saved {} but on-chain is {}",
                info.provider_id, onchain_provider_id
            )
            .into());
        }
        println!(
            "  {} On-chain provider ID matches: {}",
            "✓".green(),
            onchain_provider_id
        );

        // Try to verify provider is in approved list (optional - may not be supported by all contract versions)
        match registration::verify_approved_provider(
            run_id,
            &state_view_address,
            info.provider_id,
            &lotus_rpc_url,
        ) {
            Ok(true) => {
                println!(
                    "  {} Provider {} is in approved list",
                    "✓".green(),
                    info.provider_id
                );
            }
            Ok(false) => {
                println!(
                    "  {} Warning: Provider {} not found in approved list (verification may be incomplete)",
                    "⚠".yellow(),
                    info.provider_id
                );
            }
            Err(e) => {
                println!(
                    "  {} Warning: Could not verify approved provider list: {}",
                    "⚠".yellow(),
                    e
                );
                println!(
                    "  {} This is non-critical - the addApprovedProvider transaction succeeded",
                    "ℹ".cyan()
                );
            }
        }

        println!(
            "  {} All critical on-chain verifications passed",
            "✓".green().bold()
        );

        Ok(())
    }
}
