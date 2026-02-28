//! Export DevNet information from SetupContext.
//!
//! This module extracts data from the SetupContext and produces a
//! VersionedDevnetInfo structure that can be serialized to JSON.

use std::path::Path;

use chrono::Utc;

use crate::commands::start::step::SetupContext;
use crate::constants::USER_ACCOUNT_COUNT;
use crate::crypto::derive_ethereum_key;
use crate::crypto::mnemonic::load_mnemonic;
use crate::external_api::{
    ContractsInfo, CurioInfo, DevnetInfoV1, LotusInfo, LotusMinerInfo, UserInfo,
    VersionedDevnetInfo, YugabyteInfo, DEVNET_INFO_SCHEMA_VERSION,
};
use crate::paths;

/// Export DevNet information to a JSON file.
///
/// Extracts all relevant information from the SetupContext and writes
/// it to `devnet-info.json` in the run directory.
pub fn export_devnet_info(context: &SetupContext) -> Result<(), Box<dyn std::error::Error>> {
    let info = build_devnet_info(context)?;
    let versioned = VersionedDevnetInfo {
        version: DEVNET_INFO_SCHEMA_VERSION,
        info,
    };

    let output_path = paths::devnet_info_file(context.run_id());
    write_json_file(&output_path, &versioned)?;

    tracing::info!("Exported DevNet info to: {}", output_path.display());
    Ok(())
}

/// Build DevnetInfoV1 from SetupContext.
fn build_devnet_info(ctx: &SetupContext) -> Result<DevnetInfoV1, Box<dyn std::error::Error>> {
    Ok(DevnetInfoV1 {
        run_id: ctx.run_id().to_string(),
        start_time: Utc::now().to_rfc3339(),
        startup_duration: ctx
            .get("step_timing_total_execution_time")
            .unwrap_or_else(|| "in-progress".to_string()),
        users: build_users(ctx)?,
        contracts: build_contracts(ctx)?,
        lotus: build_lotus_info(ctx)?,
        lotus_miner: build_lotus_miner_info(ctx)?,
        pdp_sps: build_pdp_service_providers(ctx)?,
    })
}

/// Build user information from context and mnemonic.
fn build_users(ctx: &SetupContext) -> Result<Vec<UserInfo>, Box<dyn std::error::Error>> {
    let mnemonic = load_mnemonic()?;
    let seed = mnemonic.to_seed("");

    let mut users = Vec::new();
    for i in 1..=USER_ACCOUNT_COUNT {
        let name = format!("USER_{}", i);
        let user = build_single_user(ctx, &name, &seed)?;
        users.push(user);
    }
    Ok(users)
}

/// Build a single user's info.
fn build_single_user(
    ctx: &SetupContext,
    name: &str,
    seed: &[u8; 64],
) -> Result<UserInfo, Box<dyn std::error::Error>> {
    let key_name = name.to_uppercase();
    let derived = derive_ethereum_key(seed, &key_name)?;

    let evm_addr = ctx
        .get(&format!("{}_eth_address", name.to_lowercase()))
        .or_else(|| derived.eth_address.clone())
        .ok_or(format!(
            "{}_eth_address not found in context",
            name.to_lowercase()
        ))?;

    let native_addr = ctx
        .get(&format!("{}_address", name.to_lowercase()))
        .ok_or(format!(
            "{}_address not found in context",
            name.to_lowercase()
        ))?;

    Ok(UserInfo {
        name: name.to_string(),
        evm_addr,
        native_addr,
        private_key_hex: format!("0x{}", derived.private_key),
    })
}

/// Build contracts info from context.
fn build_contracts(ctx: &SetupContext) -> Result<ContractsInfo, Box<dyn std::error::Error>> {
    Ok(ContractsInfo {
        multicall3_addr: ctx
            .get("multicall3_address")
            .ok_or("Missing multicall3_address in context")?,
        mockusdfc_addr: ctx
            .get("mockusdfc_contract_address")
            .ok_or("Missing mockusdfc_contract_address in context")?,
        fwss_service_proxy_addr: ctx
            .get("foc_contract_filecoin_warm_storage_service_proxy")
            .ok_or("Missing foc_contract_filecoin_warm_storage_service_proxy in context")?,
        fwss_state_view_addr: ctx
            .get("foc_contract_filecoin_warm_storage_service_state_view")
            .ok_or("Missing foc_contract_filecoin_warm_storage_service_state_view in context")?,
        fwss_impl_addr: ctx
            .get("foc_contract_filecoin_warm_storage_service_implementation")
            .ok_or(
                "Missing foc_contract_filecoin_warm_storage_service_implementation in context",
            )?,
        pdp_verifier_proxy_addr: ctx
            .get("foc_contract_p_d_p_verifier_proxy")
            .ok_or("Missing foc_contract_p_d_p_verifier_proxy in context")?,
        pdp_verifier_impl_addr: ctx
            .get("foc_contract_p_d_p_verifier_implementation")
            .ok_or("Missing foc_contract_p_d_p_verifier_implementation in context")?,
        service_provider_registry_proxy_addr: ctx
            .get("foc_contract_service_provider_registry_proxy")
            .ok_or("Missing foc_contract_service_provider_registry_proxy in context")?,
        service_provider_registry_impl_addr: ctx
            .get("foc_contract_service_provider_registry_implementation")
            .ok_or("Missing foc_contract_service_provider_registry_implementation in context")?,
        filecoin_pay_v1_addr: ctx
            .get("foc_contract_filecoin_pay_v1_contract")
            .ok_or("Missing foc_contract_filecoin_pay_v1_contract in context")?,
        endorsements_addr: ctx
            .get("foc_contract_endorsements")
            .ok_or("Missing foc_contract_endorsements in context")?,
        session_key_registry_addr: ctx
            .get("foc_contract_session_key_registry")
            .ok_or("Missing foc_contract_session_key_registry in context")?,
    })
}

/// Build Lotus node info from context.
fn build_lotus_info(ctx: &SetupContext) -> Result<LotusInfo, Box<dyn std::error::Error>> {
    let api_port = ctx
        .get("lotus_api_port")
        .expect("lotus_api_port not found in context");
    Ok(LotusInfo {
        host_rpc_url: format!("http://localhost:{}/rpc/v1", api_port),
        container_id: ctx
            .get("lotus_container_id")
            .ok_or("Missing lotus_container_id in context")?,
        container_name: ctx
            .get("lotus_container_name")
            .ok_or("Missing lotus_container_name in context")?,
    })
}

/// Build Lotus miner info from context.
fn build_lotus_miner_info(
    ctx: &SetupContext,
) -> Result<LotusMinerInfo, Box<dyn std::error::Error>> {
    let api_port: u16 = ctx
        .get("lotus_miner_api_port")
        .and_then(|p| p.parse().ok())
        .expect("lotus_miner_api_port not found or invalid in context");

    Ok(LotusMinerInfo {
        container_id: ctx
            .get("lotus_miner_container_id")
            .ok_or("Missing lotus_miner_container_id in context")?,
        container_name: ctx
            .get("lotus_miner_container_name")
            .ok_or("Missing lotus_miner_container_name in context")?,
        api_port,
    })
}

/// Build PDP service providers info from context.
fn build_pdp_service_providers(
    ctx: &SetupContext,
) -> Result<Vec<CurioInfo>, Box<dyn std::error::Error>> {
    let active_count: usize = ctx
        .get("active_pdp_sp_count")
        .and_then(|s| s.parse().ok())
        .ok_or("active_pdp_sp_count not found or invalid in context")?;

    (1..=active_count)
        .map(|id| build_single_pdp_service_provider(ctx, id as u32))
        .collect()
}

/// Build a single PDP service provider's info.
fn build_single_pdp_service_provider(
    ctx: &SetupContext,
    provider_id: u32,
) -> Result<CurioInfo, Box<dyn std::error::Error>> {
    let eth_addr = ctx
        .get(&format!("pdp_sp_{}_eth_address", provider_id))
        .ok_or(format!(
            "pdp_sp_{}_eth_address not found in context",
            provider_id
        ))?;
    let native_addr = ctx
        .get(&format!("pdp_sp_{}_address", provider_id))
        .ok_or(format!(
            "pdp_sp_{}_address not found in context",
            provider_id
        ))?;
    let pdp_port: u16 = ctx
        .get(&format!("pdp_sp_{}_pdp_port", provider_id))
        .and_then(|p| p.parse().ok())
        .ok_or(format!(
            "pdp_sp_{}_pdp_port not found or invalid in context",
            provider_id
        ))?;

    let container_id = ctx
        .get(&format!("pdp_sp_{}_container_id", provider_id))
        .ok_or(format!(
            "pdp_sp_{}_container_id not found in context",
            provider_id
        ))?;
    let container_name = ctx
        .get(&format!("pdp_sp_{}_container_name", provider_id))
        .ok_or(format!(
            "pdp_sp_{}_container_name not found in context",
            provider_id
        ))?;

    let is_approved = ctx
        .get(&format!("pdp_sp_{}_is_approved", provider_id))
        .and_then(|v| v.parse::<bool>().ok())
        .ok_or(format!(
            "pdp_sp_{}_is_approved not found or invalid in context",
            provider_id
        ))?;

    let is_endorsed = ctx
        .get(&format!("pdp_sp_{}_is_endorsed", provider_id))
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    let yugabyte = build_yugabyte_info(ctx, provider_id)?;

    Ok(CurioInfo {
        provider_id,
        eth_addr,
        native_addr,
        pdp_service_url: format!("http://localhost:{}", pdp_port),
        container_id,
        container_name,
        is_approved,
        is_endorsed,
        yugabyte,
    })
}

/// Build YugabyteDB info for a provider.
fn build_yugabyte_info(
    ctx: &SetupContext,
    provider_id: u32,
) -> Result<YugabyteInfo, Box<dyn std::error::Error>> {
    let web_ui_port: u16 = ctx
        .get(&format!("yugabyte_{}_web_ui_port", provider_id))
        .and_then(|p| p.parse().ok())
        .ok_or(format!(
            "yugabyte_{}_web_ui_port not found or invalid in context",
            provider_id
        ))?;

    let master_rpc_port: u16 = ctx
        .get(&format!("yugabyte_{}_master_rpc_port", provider_id))
        .and_then(|p| p.parse().ok())
        .ok_or(format!(
            "yugabyte_{}_master_rpc_port not found or invalid in context",
            provider_id
        ))?;

    let ysql_port: u16 = ctx
        .get(&format!("yugabyte_{}_ysql_port", provider_id))
        .and_then(|p| p.parse().ok())
        .ok_or(format!(
            "yugabyte_{}_ysql_port not found or invalid in context",
            provider_id
        ))?;

    Ok(YugabyteInfo {
        web_ui_url: format!("http://localhost:{}", web_ui_port),
        master_rpc_port,
        ysql_port,
    })
}

/// Write a serializable struct to a JSON file.
fn write_json_file<T: serde::Serialize>(
    path: &Path,
    data: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(path, json)?;
    Ok(())
}
