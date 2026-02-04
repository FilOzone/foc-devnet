//! Endorsement step implementation.

use super::endorsement::{
    endorse_provider, verify_endorsement, EndorseParams, VerifyEndorsementParams,
};
use crate::commands::start::foc_deploy::contract_addresses::ContractAddresses;
use crate::commands::start::lotus_utils;
use crate::commands::start::step::{SetupContext, Step};
use crate::docker::containers::lotus_container_name;
use crate::docker::core::container_is_running;
use std::error::Error;
use std::path::PathBuf;
use tracing::{info, warn};

/// Step for endorsing PDP service providers
pub struct EndorsementStep {
    #[allow(dead_code)]
    run_dir: PathBuf,
    endorsed_sp_count: usize,
    #[allow(dead_code)]
    active_sp_count: usize,
}

impl EndorsementStep {
    /// Create a new EndorsementStep
    pub fn new(
        _volumes_dir: PathBuf,
        run_dir: PathBuf,
        endorsed_sp_count: usize,
        active_sp_count: usize,
    ) -> Self {
        Self {
            run_dir,
            endorsed_sp_count,
            active_sp_count,
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

    /// Get deployer address from context
    fn get_deployer_address(context: &SetupContext) -> Result<String, Box<dyn Error>> {
        context
            .get("deployer_foc_eth_address")
            .ok_or("deployer_foc_eth_address not found in context".into())
    }
}

impl Step for EndorsementStep {
    fn name(&self) -> &str {
        "PDP Provider Endorsement"
    }

    fn pre_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        info!("Pre-checking {}", self.name());

        Self::check_lotus_running(context)?;
        info!("Lotus is running");

        let deployer_address = Self::get_deployer_address(context)?;
        info!("Deployer address: {}", deployer_address);

        let contract_addresses = Self::load_contract_addresses(context)?;
        let endorsements_address = contract_addresses
            .foc_contracts
            .get("endorsements")
            .ok_or("Endorsements contract address not found")?;
        info!("Endorsements contract: {}", endorsements_address);

        for sp_index in 1..=self.endorsed_sp_count {
            let pdp_key = format!("pdp_sp_{}_address", sp_index);
            let provider_id_key = format!("pdp_sp_{}_provider_id", sp_index);
            let approved_key = format!("pdp_sp_{}_is_approved", sp_index);

            let sp_address = context
                .get(&pdp_key)
                .ok_or(format!("{} not found in context", pdp_key))?;

            let provider_id: u64 = context
                .get(&provider_id_key)
                .ok_or(format!("{} not found in context", provider_id_key))?
                .parse()?;

            let is_approved = context
                .get(&approved_key)
                .and_then(|v| v.parse::<bool>().ok())
                .unwrap_or(false);

            if !is_approved {
                warn!("PDP SP {} is not approved, cannot endorse", sp_index);
                return Err(format!(
                    "PDP SP {} must be approved before it can be endorsed",
                    sp_index
                )
                .into());
            }

            info!("PDP SP {} ready for endorsement:", sp_index);
            info!("  Address: {}", sp_address);
            info!("  Provider ID: {}", provider_id);
        }

        Ok(())
    }

    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        if self.endorsed_sp_count == 0 {
            info!("No providers to endorse (endorsed_pdp_sp_count = 0), skipping");
            return Ok(());
        }

        Self::check_lotus_running(context)?;

        let run_id = context.run_id();
        let lotus_rpc_url = lotus_utils::get_lotus_rpc_url(context)?;

        let contract_addresses = Self::load_contract_addresses(context)?;
        let endorsements_address = contract_addresses
            .foc_contracts
            .get("endorsements")
            .ok_or("Endorsements contract address not found")?
            .clone();

        let deployer_address = Self::get_deployer_address(context)?;

        // Get deployer private key
        let deployer_private_key =
            crate::commands::start::foc_deployer::get_private_key(&deployer_address, "")?;

        info!(
            "Endorsing {} provider(s) in ProviderIdSet contract...",
            self.endorsed_sp_count
        );

        for sp_index in 1..=self.endorsed_sp_count {
            let provider_id_key = format!("pdp_sp_{}_provider_id", sp_index);
            let provider_id: u64 = context
                .get(&provider_id_key)
                .ok_or(format!("{} not found in context", provider_id_key))?
                .parse()?;

            info!("Endorsing Provider {} (ID: {})...", sp_index, provider_id);

            let params = EndorseParams {
                run_id: run_id.to_string(),
                provider_id,
                endorsements_contract_address: endorsements_address.clone(),
                deployer_private_key: deployer_private_key.clone(),
                lotus_rpc_url: lotus_rpc_url.clone(),
            };

            let tx_hash = endorse_provider(params, context)?;

            let tx_key = format!("pdp_sp_{}_endorsement_tx", sp_index);
            context.set(&tx_key, tx_hash);

            info!("Provider {} endorsed successfully", sp_index);
        }

        info!(
            "All {} provider(s) endorsed successfully",
            self.endorsed_sp_count
        );

        Ok(())
    }

    fn post_execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        if self.endorsed_sp_count == 0 {
            return Ok(());
        }

        info!("Verifying endorsements...");

        let lotus_rpc_url = lotus_utils::get_lotus_rpc_url(context)?;
        let contract_addresses = Self::load_contract_addresses(context)?;
        let endorsements_address = contract_addresses
            .foc_contracts
            .get("endorsements")
            .ok_or("Endorsements contract address not found")?
            .clone();

        for sp_index in 1..=self.endorsed_sp_count {
            let provider_id_key = format!("pdp_sp_{}_provider_id", sp_index);
            let provider_id: u64 = context
                .get(&provider_id_key)
                .ok_or(format!("{} not found in context", provider_id_key))?
                .parse()?;

            let params = VerifyEndorsementParams {
                provider_id,
                endorsements_contract_address: endorsements_address.clone(),
                lotus_rpc_url: lotus_rpc_url.clone(),
            };

            let is_endorsed = verify_endorsement(params, context)?;

            if !is_endorsed {
                return Err(format!(
                    "Verification failed: Provider {} is not endorsed in contract",
                    sp_index
                )
                .into());
            }

            let endorsed_key = format!("pdp_sp_{}_endorsed", sp_index);
            context.set(&endorsed_key, "true".to_string());

            info!("Provider {} endorsement verified ✓", sp_index);
        }

        info!(
            "All {} endorsements verified successfully",
            self.endorsed_sp_count
        );

        Ok(())
    }
}
