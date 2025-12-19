//! PDP Service Provider step implementation.

use super::super::step::{SetupContext, Step};
use super::provider_id::ProviderIdInfo;
use super::registration;
use crate::commands::start::foc_deploy::contract_addresses::ContractAddresses;
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use std::error::Error;
use std::path::PathBuf;
use tracing::info;

/// Step for registering PDP service provider
pub struct PdpSpRegistrationStep {
    #[allow(dead_code)]
    run_dir: PathBuf,
    /// Number of PDP SPs to activate (1-5)
    active_sp_count: usize,
    /// Number of PDP SPs to approve in registry
    approved_sp_count: usize,
}

impl PdpSpRegistrationStep {
    /// Create a new PDPServiceProviderStep
    pub fn new(
        _volumes_dir: PathBuf,
        run_dir: PathBuf,
        active_sp_count: usize,
        approved_sp_count: usize,
    ) -> Self {
        Self {
            run_dir,
            active_sp_count,
            approved_sp_count,
        }
    }

    /// Check if Lotus is running
    fn check_lotus_running(context: &SetupContext) -> Result<(), Box<dyn Error>> {
        let run_id = context.run_id();
        let container_name = lotus_container_name(run_id);
        if !container_is_running(&container_name)? {
            return Err("Lotus container is not running.".into());
        }
        Ok(())
    }

    /// Load contract addresses from state
    fn load_contract_addresses(
        context: &SetupContext,
    ) -> Result<ContractAddresses, Box<dyn Error>> {
        let run_id = context.run_id();
        ContractAddresses::load(run_id)
            .map_err(|e| format!("Failed to load contract addresses: {}", e).into())
    }

    /// Get required addresses from context
    fn get_required_addresses(context: &SetupContext) -> Result<(String, String), Box<dyn Error>> {
        let deployer_foc_address = context
            .get("deployer_foc_address")
            .ok_or("DEPLOYER_FOC address not found in context")?;

        let deployer_foc_eth_address = context
            .get("deployer_foc_eth_address")
            .ok_or("DEPLOYER_FOC Ethereum address not found in context")?;

        Ok((
            deployer_foc_address.clone(),
            deployer_foc_eth_address.clone(),
        ))
    }
}

impl Step for PdpSpRegistrationStep {
    fn name(&self) -> &str {
        "PDP Service Provider Registration"
    }

    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        info!("Pre-checking {}", self.name());

        // Check if Lotus is running
        Self::check_lotus_running(context)?;
        info!("Lotus is running");

        // Check if required addresses are available
        let (deployer_foc_address, deployer_foc_eth_address) =
            Self::get_required_addresses(context)?;
        info!("DEPLOYER_FOC address: {}", deployer_foc_address);
        info!("DEPLOYER_FOC ETH address: {}", deployer_foc_eth_address);

        // Check if all SP addresses are available
        for sp_index in 1..=self.active_sp_count {
            let pdp_key = format!("pdp_sp_{}_address", sp_index);
            let eth_key = format!("pdp_sp_{}_eth_address", sp_index);
            let port_key = format!("curio_sp_{}_pdp_port", sp_index);

            let sp_address = context
                .get(&pdp_key)
                .ok_or(format!("{} not found in context", pdp_key))?;
            let sp_eth_address = context
                .get(&eth_key)
                .ok_or(format!("{} not found in context", eth_key))?;
            let pdp_port: u16 = context
                .get(&port_key)
                .ok_or(format!(
                    "{} not found in context - Curio must be started first",
                    port_key
                ))?
                .parse()?;

            info!("PDP SP {} address: {}", sp_index, sp_address);
            info!("PDP SP {} ETH address: {}", sp_index, sp_eth_address);
            info!("PDP SP {} PDP port: {}", sp_index, pdp_port);
        }

        // Check if contract addresses are available
        let contract_addresses = Self::load_contract_addresses(context)?;
        let registry_address = contract_addresses
            .foc_contracts
            .get("service_provider_registry_proxy")
            .ok_or("service_provider_registry_proxy address not found")?;
        info!("ServiceProviderRegistry: {}", registry_address);

        let warm_storage_address = contract_addresses
            .foc_contracts
            .get("filecoin_warm_storage_service_proxy")
            .ok_or("filecoin_warm_storage_service_proxy address not found")?;
        info!("WarmStorageService: {}", warm_storage_address);

        Ok(())
    }

    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        Self::check_lotus_running(context)?;

        let run_id = context.run_id();
        let lotus_rpc_url = crate::commands::start::lotus_utils::get_lotus_rpc_url(context)?;

        let contract_addresses = Self::load_contract_addresses(context)?;
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

        // Get deployer addresses (for approval)
        let deployer_foc_address = context
            .get("deployer_foc_address")
            .ok_or("DEPLOYER_FOC address not found in context")?;
        let deployer_foc_eth_address = context
            .get("deployer_foc_eth_address")
            .ok_or("DEPLOYER_FOC Ethereum address not found in context")?;

        // Collect SP registration data
        let mut sp_data = Vec::new();
        for sp_index in 1..=self.active_sp_count {
            let pdp_key = format!("pdp_sp_{}_address", sp_index);
            let eth_key = format!("pdp_sp_{}_eth_address", sp_index);
            let port_key = format!("curio_sp_{}_pdp_port", sp_index);

            let sp_address = context
                .get(&pdp_key)
                .ok_or(format!("{} not found in context", pdp_key))?;
            let sp_eth_address = context
                .get(&eth_key)
                .ok_or(format!("{} not found in context", eth_key))?;
            let pdp_port: u16 = context
                .get(&port_key)
                .ok_or(format!(
                    "{} not found in context - Curio must be started first",
                    port_key
                ))?
                .parse()?;

            // Determine if this SP should be approved (only first N SPs)
            let should_approve = sp_index <= self.approved_sp_count;
            sp_data.push((
                sp_index,
                sp_address.clone(),
                sp_eth_address.clone(),
                pdp_port,
                should_approve,
            ));
        }

        // Register all SPs sequentially
        let mut errors = Vec::new();
        let mut provider_ids = Vec::new();

        for (sp_index, sp_address, sp_eth_address, pdp_port, should_approve) in sp_data {
            let service_url = format!("http://localhost:{}", pdp_port);

            match registration::register_single_provider(
                run_id,
                &registry_address,
                &sp_address,
                &sp_eth_address,
                &mock_usdfc_address,
                &lotus_rpc_url,
                &service_url,
                sp_index,
                context,
            ) {
                Ok(provider_id) => {
                    // Only approve if within approved count
                    if should_approve {
                        if let Err(e) = registration::add_to_approved_list(
                            run_id,
                            &warm_storage_address,
                            provider_id,
                            deployer_foc_address.as_str(),
                            deployer_foc_eth_address.as_str(),
                            &lotus_rpc_url,
                            sp_index,
                            context,
                        ) {
                            errors.push(format!("SP {} approval failed: {}", sp_index, e));
                        } else {
                            provider_ids.push((sp_index, provider_id));
                            info!(
                                "PDP SP {} registered and approved (Provider ID: {}, URL: {})",
                                sp_index, provider_id, service_url
                            );
                        }
                    } else {
                        // Registered but not approved
                        provider_ids.push((sp_index, provider_id));
                        info!(
                            "PDP SP {} registered (not approved, Provider ID: {}, URL: {})",
                            sp_index, provider_id, service_url
                        );
                    }
                }
                Err(e) => {
                    errors.push(format!("SP {} registration failed: {}", sp_index, e));
                }
            }
        }

        // Check for errors
        if !errors.is_empty() {
            return Err(format!(
                "Failed to register some providers:\n{}",
                errors.join("\n")
            )
            .into());
        }

        // Store all provider IDs
        for (sp_index, provider_id) in &provider_ids {
            let sp_key_prefix = format!("pdp_sp_{}", sp_index); // 1-indexed keys
            let sp_eth_address = context
                .get(&format!("{}_eth_address", sp_key_prefix))
                .ok_or("SP eth address not found")?;

            let info = ProviderIdInfo {
                provider_id: *provider_id,
                provider_address: sp_eth_address.clone(),
                payee_address: sp_eth_address.clone(),
            };
            info.save(run_id, *sp_index)?;
        }

        info!(
            "All {} PDP SP(s) registered successfully",
            self.active_sp_count
        );

        Ok(())
    }

    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        Self::check_lotus_running(context)?;

        let run_id = context.run_id();
        let lotus_rpc_url = crate::commands::start::lotus_utils::get_lotus_rpc_url(context)?;

        let contract_addresses = Self::load_contract_addresses(context)?;
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

        for sp_index in 1..=self.active_sp_count {
            info!("Verifying PDP SP {}...", sp_index);
            // Verify provider ID file exists and is valid
            let info = ProviderIdInfo::load(run_id, sp_index)?;
            info!("Provider ID file exists: {}", info.provider_id);

            // Verify on-chain provider ID matches the saved one
            let onchain_provider_id = registration::verify_provider_id_by_address(
                run_id,
                &registry_address,
                &info.provider_address,
                &lotus_rpc_url,
                context,
            )?;
            if onchain_provider_id != info.provider_id {
                return Err(format!(
                    "Provider ID mismatch: saved {} but on-chain is {}",
                    info.provider_id, onchain_provider_id
                )
                .into());
            }
            info!("On-chain provider ID matches: {}", onchain_provider_id);

            if sp_index < self.approved_sp_count {
                // Verify provider is approved
                let is_approved = registration::verify_approved_provider(
                    run_id,
                    &state_view_address,
                    info.provider_id,
                    &lotus_rpc_url,
                    context,
                )?;
                if !is_approved {
                    return Err(format!(
                        "Provider {} is not in the approved list but should be",
                        info.provider_id
                    )
                    .into());
                }
                info!("Provider {} is approved", info.provider_id);
            }
        }

        // Verify there's exactly the expected number of providers on-chain
        let provider_count = registration::verify_provider_count(
            run_id,
            &registry_address,
            &lotus_rpc_url,
            context,
        )?;
        if provider_count != self.active_sp_count as u64 {
            return Err(format!(
                "Expected exactly {} providers on-chain, found {}",
                self.active_sp_count, provider_count
            )
            .into());
        }
        info!("Provider count on-chain: {}", provider_count);

        info!("All critical on-chain verifications passed");

        Ok(())
    }
}
