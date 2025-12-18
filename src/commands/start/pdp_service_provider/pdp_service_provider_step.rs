//! PDP Service Provider step implementation.

use super::super::step::{Step, StepContext};
use super::provider_id::ProviderIdInfo;
use super::registration;
use crate::commands::start::foc_deploy::contract_addresses::ContractAddresses;
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use crossterm::style::Stylize;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// Step for registering PDP service provider
pub struct PdpSpRegistrationStep {
    #[allow(dead_code)]
    logs_dir: PathBuf,
    /// Number of PDP SPs to activate (1-5)
    active_sp_count: usize,
    /// Number of PDP SPs to approve in registry
    approved_sp_count: usize,
}

impl PdpSpRegistrationStep {
    /// Create a new PDPServiceProviderStep
    pub fn new(
        _volumes_dir: PathBuf,
        logs_dir: PathBuf,
        active_sp_count: usize,
        approved_sp_count: usize,
    ) -> Self {
        Self {
            logs_dir,
            active_sp_count,
            approved_sp_count,
        }
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
    fn get_required_addresses(context: &StepContext) -> Result<(String, String), Box<dyn Error>> {
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

    fn pre_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        println!(
            "{} {}",
            "Pre-checking".cyan().bold(),
            self.name().cyan().bold()
        );

        // Check if Lotus is running
        Self::check_lotus_running(context)?;
        println!("  {} Lotus is running", "✓".green());

        // Check if required addresses are available
        let (deployer_foc_address, deployer_foc_eth_address) =
            Self::get_required_addresses(context)?;
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

            println!(
                "  {} PDP SP {} address: {}",
                "✓".green(),
                sp_index,
                sp_address
            );
            println!(
                "  {} PDP SP {} ETH address: {}",
                "✓".green(),
                sp_index,
                sp_eth_address
            );
            println!(
                "  {} PDP SP {} PDP port: {}",
                "✓".green(),
                sp_index,
                pdp_port
            );
        }

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

    fn execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        use super::super::lotus_utils::get_lotus_rpc_url;

        println!(
            "{} {}",
            "Executing".green().bold(),
            self.name().green().bold()
        );

        // Determine how many SPs to register
        let num_sps = self.active_sp_count;

        println!(
            "  {} Registering {} PDP Service Provider(s) in parallel...",
            "⚡".cyan(),
            num_sps
        );

        // Get run ID
        let run_id = context.run_id().ok_or("Run ID not found in context")?;

        // Get Lotus RPC URL with dynamic port
        let lotus_rpc_url = get_lotus_rpc_url(context)?;

        // Load contract addresses (shared for all SPs)
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

        // Get deployer addresses (for approval)
        let deployer_foc_address = context
            .get("deployer_foc_address")
            .ok_or("DEPLOYER_FOC address not found in context")?;
        let deployer_foc_eth_address = context
            .get("deployer_foc_eth_address")
            .ok_or("DEPLOYER_FOC Ethereum address not found in context")?;

        // Collect SP registration data
        let mut sp_data = Vec::new();
        for sp_index in 1..=num_sps {
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

        // Register all SPs in parallel
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let provider_ids: Arc<Mutex<Vec<(usize, u64)>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();

        for (sp_index, sp_address, sp_eth_address, pdp_port, should_approve) in sp_data {
            let run_id = run_id.to_string();
            let registry_address = registry_address.clone();
            let mock_usdfc_address = mock_usdfc_address.clone();
            let lotus_rpc_url = lotus_rpc_url.clone();
            let warm_storage_address = warm_storage_address.clone();
            let deployer_foc_address = deployer_foc_address.clone();
            let deployer_foc_eth_address = deployer_foc_eth_address.clone();
            let errors_clone = Arc::clone(&errors);
            let provider_ids_clone = Arc::clone(&provider_ids);

            let handle = thread::spawn(move || {
                let service_url = format!("http://localhost:{}", pdp_port);

                match registration::register_single_provider(
                    &run_id,
                    &registry_address,
                    &sp_address,
                    &sp_eth_address,
                    &mock_usdfc_address,
                    &lotus_rpc_url,
                    &service_url,
                    sp_index,
                ) {
                    Ok(provider_id) => {
                        // Only approve if within approved count
                        if should_approve {
                            if let Err(e) = registration::add_to_approved_list(
                                &run_id,
                                &warm_storage_address,
                                provider_id,
                                &deployer_foc_address,
                                &deployer_foc_eth_address,
                                &lotus_rpc_url,
                            ) {
                                errors_clone
                                    .lock()
                                    .unwrap()
                                    .push(format!("SP {} approval failed: {}", sp_index, e));
                            } else {
                                provider_ids_clone
                                    .lock()
                                    .unwrap()
                                    .push((sp_index, provider_id));
                                println!(
                                    "  {} PDP SP {} registered and approved (Provider ID: {}, URL: {})",
                                    "✓".green(),
                                    sp_index,
                                    provider_id,
                                    service_url
                                );
                            }
                        } else {
                            // Registered but not approved
                            provider_ids_clone
                                .lock()
                                .unwrap()
                                .push((sp_index, provider_id));
                            println!(
                                "  {} PDP SP {} registered (not approved, Provider ID: {}, URL: {})",
                                "⚠".yellow(),
                                sp_index,
                                provider_id,
                                service_url
                            );
                        }
                    }
                    Err(e) => {
                        errors_clone
                            .lock()
                            .unwrap()
                            .push(format!("SP {} registration failed: {}", sp_index, e));
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().map_err(|_| "Registration thread panicked")?;
        }

        // Check for errors
        let error_list = errors.lock().unwrap();
        if !error_list.is_empty() {
            return Err(format!(
                "Failed to register some providers:\n{}",
                error_list.join("\n")
            )
            .into());
        }

        // Store first provider ID (for backward compatibility)
        let provider_ids_list = provider_ids.lock().unwrap();
        if let Some((sp_index, first_provider_id)) = provider_ids_list.first() {
            let sp_key_prefix = format!("pdp_sp_{}", sp_index); // 1-indexed keys
            let sp_eth_address = context
                .get(&format!("{}_eth_address", sp_key_prefix))
                .ok_or("SP eth address not found")?;

            let info = ProviderIdInfo {
                provider_id: *first_provider_id,
                provider_address: sp_eth_address.clone(),
                payee_address: sp_eth_address.clone(),
            };
            info.save(*sp_index)?;
        }

        println!(
            "  {} All {} PDP SP(s) registered successfully",
            "✓".green(),
            num_sps
        );

        Ok(())
    }

    fn post_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
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

        for sp_index in 1..=self.active_sp_count {
            println!("  {} Verifying PDP SP {}...", "🔍".cyan(), sp_index);
            // Verify provider ID file exists and is valid
            let info = ProviderIdInfo::load(sp_index)?;
            println!(
                "  {} Provider ID file exists: {}",
                "✓".green(),
                info.provider_id
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

            if sp_index < self.approved_sp_count {
                // Verify provider is approved
                let is_approved = registration::verify_approved_provider(
                    run_id,
                    &state_view_address,
                    info.provider_id,
                    &lotus_rpc_url,
                )?;
                if !is_approved {
                    return Err(format!(
                        "Provider {} is not in the approved list but should be",
                        info.provider_id
                    )
                    .into());
                }
                println!(
                    "  {} Provider {} is approved",
                    "✓".green(),
                    info.provider_id
                );
            }
        }

        // Verify there's exactly the expected number of providers on-chain
        let provider_count =
            registration::verify_provider_count(run_id, &registry_address, &lotus_rpc_url)?;
        if provider_count != self.active_sp_count as u64 {
            return Err(format!(
                "Expected exactly {} providers on-chain, found {}",
                self.active_sp_count, provider_count
            )
            .into());
        }
        println!(
            "  {} Provider count on-chain: {}",
            "✓".green(),
            provider_count
        );

        println!(
            "  {} All critical on-chain verifications passed",
            "✓".green().bold()
        );

        Ok(())
    }
}
