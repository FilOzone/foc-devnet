//! Export DevNet information from SetupContext.
//!
//! This module extracts data from the SetupContext and produces a
//! VersionedDevnetInfo structure that can be serialized to JSON.

use std::path::Path;

use chrono::Utc;

use crate::commands::start::step::SetupContext;
use crate::crypto::derive_ethereum_key;
use crate::crypto::mnemonic::load_mnemonic;
use crate::external_api::{
    ContractsInfo, CurioInfo, DevnetInfoV1, LotusInfo, LotusMinerInfo, UserInfo,
    VersionedDevnetInfo, YugabyteInfo, DEVNET_INFO_FILENAME, DEVNET_INFO_SCHEMA_VERSION,
};

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

    let output_path = context.run_dir().join(DEVNET_INFO_FILENAME);
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
            .unwrap_or_else(|| "unknown".to_string()),
        users: build_users(ctx)?,
        contracts: build_contracts(ctx),
        lotus: build_lotus_info(ctx),
        lotus_miner: build_lotus_miner_info(ctx),
        pdp_sps: build_curio_providers(ctx),
    })
}

/// Build user information from context and mnemonic.
fn build_users(ctx: &SetupContext) -> Result<Vec<UserInfo>, Box<dyn std::error::Error>> {
    let mnemonic = load_mnemonic()?;
    let seed = mnemonic.to_seed("");

    let mut users = Vec::new();
    for i in 1..=3 {
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
        .unwrap_or_default();

    let native_addr = ctx
        .get(&format!("{}_address", name.to_lowercase()))
        .unwrap_or_else(|| derived.native_address.clone());

    // Format USDFC balance: 100,000 tokens with 18 decimals = 100000000000000000000000 wei
    // But display in human-readable form with decimals
    let mockusdfc_wei = "100000000000000000000000"; // 100,000 USDFC
    let mockusdfc_formatted = format_token_balance(mockusdfc_wei);

    Ok(UserInfo {
        name: name.to_string(),
        evm_addr,
        native_addr,
        native_balance_tfil: "1000".to_string(), // Default funding amount
        mockusdfc_balance: mockusdfc_formatted,
        private_key_hex: format!("0x{}", derived.private_key),
    })
}

/// Format token balance from wei to human-readable form with 18 decimals
fn format_token_balance(wei: &str) -> String {
    if wei.len() <= 18 {
        // Less than 1 token
        let padded = format!("{:0>18}", wei);
        format!("0.{}", padded)
    } else {
        let split_point = wei.len() - 18;
        let whole = &wei[..split_point];
        let fraction = &wei[split_point..];
        format!("{}.{}", whole, fraction)
    }
}

/// Build contracts info from context.
fn build_contracts(ctx: &SetupContext) -> ContractsInfo {
    ContractsInfo {
        multicall3_addr: ctx.get("multicall3_address").unwrap_or_default(),
        mockusdfc_addr: ctx.get("mockusdfc_contract_address").unwrap_or_default(),
        fwss_service_proxy_addr: ctx
            .get("foc_contract_filecoin_warm_storage_service_proxy")
            .unwrap_or_default(),
        fwss_state_view_addr: ctx
            .get("foc_contract_filecoin_warm_storage_service_state_view")
            .unwrap_or_default(),
        fwss_impl_addr: ctx
            .get("foc_contract_filecoin_warm_storage_service_implementation")
            .unwrap_or_default(),
        pdp_verifier_proxy_addr: ctx
            .get("foc_contract_p_d_p_verifier_proxy")
            .unwrap_or_default(),
        pdp_verifier_impl_addr: ctx
            .get("foc_contract_p_d_p_verifier_implementation")
            .unwrap_or_default(),
        service_provider_registry_proxy_addr: ctx
            .get("foc_contract_service_provider_registry_proxy")
            .unwrap_or_default(),
        service_provider_registry_impl_addr: ctx
            .get("foc_contract_service_provider_registry_implementation")
            .unwrap_or_default(),
        filecoin_pay_v1_addr: ctx
            .get("foc_contract_filecoin_pay_v1_contract")
            .unwrap_or_default(),
        endorsements_addr: ctx
            .get("foc_contract_endorsements")
            .unwrap_or_default(),
    }
}

/// Build Lotus node info from context.
fn build_lotus_info(ctx: &SetupContext) -> LotusInfo {
    let api_port = ctx
        .get("lotus_api_port")
        .unwrap_or_else(|| "1234".to_string());
    LotusInfo {
        host_rpc_url: format!("http://localhost:{}/rpc/v1", api_port),
        container_id: ctx.get("lotus_container_id").unwrap_or_default(),
        container_name: ctx.get("lotus_container_name").unwrap_or_default(),
    }
}

/// Build Lotus miner info from context.
fn build_lotus_miner_info(ctx: &SetupContext) -> LotusMinerInfo {
    let api_port: u16 = ctx
        .get("lotus_miner_api_port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(2345);

    LotusMinerInfo {
        container_id: ctx.get("lotus_miner_container_id").unwrap_or_default(),
        container_name: ctx.get("lotus_miner_container_name").unwrap_or_default(),
        api_port,
    }
}

/// Build Curio providers info from context.
fn build_curio_providers(ctx: &SetupContext) -> Vec<CurioInfo> {
    let active_count: usize = ctx
        .get("active_pdp_sp_count")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    (1..=active_count)
        .filter_map(|id| build_single_curio_provider(ctx, id as u32))
        .collect()
}

/// Build a single Curio provider's info.
fn build_single_curio_provider(ctx: &SetupContext, provider_id: u32) -> Option<CurioInfo> {
    let eth_addr = ctx.get(&format!("pdp_sp_{}_eth_address", provider_id))?;
    let native_addr = ctx
        .get(&format!("pdp_sp_{}_address", provider_id))
        .unwrap_or_default();
    let pdp_port: u16 = ctx
        .get(&format!("curio_sp_{}_pdp_port", provider_id))
        .and_then(|p| p.parse().ok())
        .unwrap_or(4702);

    let container_id = ctx
        .get(&format!("curio_sp_{}_container_id", provider_id))
        .unwrap_or_default();
    let container_name = ctx
        .get(&format!("curio_sp_{}_container_name", provider_id))
        .unwrap_or_default();

    let yugabyte = build_yugabyte_info(ctx, provider_id);

    Some(CurioInfo {
        provider_id,
        eth_addr,
        native_addr,
        pdp_service_url: format!("http://localhost:{}", pdp_port),
        container_id,
        container_name,
        yugabyte,
    })
}

/// Build YugabyteDB info for a provider.
fn build_yugabyte_info(ctx: &SetupContext, provider_id: u32) -> YugabyteInfo {
    let web_ui_port: u16 = ctx
        .get(&format!("yugabyte_{}_web_ui_port", provider_id))
        .and_then(|p| p.parse().ok())
        .unwrap_or(15433);

    let master_rpc_port: u16 = ctx
        .get(&format!("yugabyte_{}_master_rpc_port", provider_id))
        .and_then(|p| p.parse().ok())
        .unwrap_or(7100);

    let ysql_port: u16 = ctx
        .get(&format!("yugabyte_{}_ysql_port", provider_id))
        .and_then(|p| p.parse().ok())
        .unwrap_or(5433);

    YugabyteInfo {
        web_ui_url: format!("http://localhost:{}", web_ui_port),
        master_rpc_port,
        ysql_port,
    }
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
