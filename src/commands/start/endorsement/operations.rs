//! Business logic for provider endorsement operations.

use super::constants::{
    ENDORSEMENT_CONTAINER_PREFIX, ENDORSEMENT_GAS_LIMIT, ENDORSEMENT_TX_WAIT_SECS,
};
use crate::commands::start::step::SetupContext;
use crate::constants::BUILDER_DOCKER_IMAGE;
use crate::docker::command_logger::run_and_log_command_strings;
use std::error::Error;
use std::thread;
use std::time::Duration;
use tracing::info;

/// Parameters for endorsing a provider
pub struct EndorseParams {
    pub run_id: String,
    pub provider_id: u64,
    pub endorsements_contract_address: String,
    pub deployer_private_key: String,
    pub lotus_rpc_url: String,
}

/// Endorse a provider in the ProviderIdSet contract.
pub fn endorse_provider(
    params: EndorseParams,
    context: &SetupContext,
) -> Result<String, Box<dyn Error>> {
    let container_name = format!(
        "{}-{}-{}",
        ENDORSEMENT_CONTAINER_PREFIX, params.run_id, params.provider_id
    );

    let label = format!("Provider {}", params.provider_id);
    info!("Endorsing {} in ProviderIdSet...", label);

    let cast_cmd = format!(
        r#"cast send {} "addProviderId(uint256)" {} \
        --rpc-url {} \
        --private-key {} \
        --gas-limit {}"#,
        params.endorsements_contract_address,
        params.provider_id,
        params.lotus_rpc_url,
        params.deployer_private_key,
        ENDORSEMENT_GAS_LIMIT
    );

    let args: Vec<String> = vec![
        "run".to_string(),
        "--name".to_string(),
        container_name.clone(),
        "-u".to_string(),
        "foc-user".to_string(),
        "--network".to_string(),
        "host".to_string(),
        BUILDER_DOCKER_IMAGE.to_string(),
        "bash".to_string(),
        "-c".to_string(),
        cast_cmd,
    ];

    let key = format!("pdp_endorse_{}", params.provider_id);
    let output = run_and_log_command_strings("docker", &args, context, &key)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to endorse provider: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tx_hash = extract_tx_hash(&stdout)
        .ok_or("Failed to extract transaction hash from endorsement output")?;

    info!("Endorsement transaction: {}", tx_hash);
    thread::sleep(Duration::from_secs(ENDORSEMENT_TX_WAIT_SECS));

    verify_transaction_status(&params.lotus_rpc_url, &tx_hash, params.provider_id, context)?;

    info!("{} endorsed successfully", label);

    Ok(tx_hash)
}

/// Extract transaction hash from cast send output.
fn extract_tx_hash(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("transactionHash") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(hash) = parts.last() {
                return Some(hash.to_string());
            }
        }
    }
    None
}

/// Verify that the endorsement transaction succeeded.
fn verify_transaction_status(
    rpc_url: &str,
    tx_hash: &str,
    provider_id: u64,
    context: &SetupContext,
) -> Result<(), Box<dyn Error>> {
    let container_name = format!("foc-verify-endorse-{}", provider_id);

    let cast_cmd = format!(r#"cast receipt {} --rpc-url {} --json"#, tx_hash, rpc_url);

    let args: Vec<String> = vec![
        "run".to_string(),
        "--name".to_string(),
        container_name.clone(),
        "-u".to_string(),
        "foc-user".to_string(),
        "--network".to_string(),
        "host".to_string(),
        BUILDER_DOCKER_IMAGE.to_string(),
        "bash".to_string(),
        "-c".to_string(),
        cast_cmd,
    ];

    let key = format!("pdp_verify_endorse_tx_{}", provider_id);
    let output = run_and_log_command_strings("docker", &args, context, &key)?;

    if !output.status.success() {
        return Err("Failed to get transaction receipt".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let receipt: serde_json::Value = serde_json::from_str(&stdout)?;

    let status = receipt["status"]
        .as_str()
        .ok_or("Transaction status not found in receipt")?;

    if status != "0x1" {
        return Err(format!(
            "Endorsement transaction failed with status {}. Expected 0x1 (success). Provider {} not endorsed.",
            status, provider_id
        )
        .into());
    }

    Ok(())
}

/// Parameters for getting all provider IDs
pub struct GetProviderIdsParams {
    pub endorsements_contract_address: String,
    pub lotus_rpc_url: String,
}

/// Get all endorsed provider IDs from the contract.
/// Returns the provider IDs in the order they were inserted.
pub fn get_all_provider_ids(
    params: GetProviderIdsParams,
    context: &SetupContext,
) -> Result<Vec<u64>, Box<dyn Error>> {
    let container_name = "foc-get-provider-ids";

    let cast_cmd = format!(
        r#"cast call {} "getProviderIds()(uint256[])" --rpc-url {}"#,
        params.endorsements_contract_address, params.lotus_rpc_url
    );

    let args: Vec<String> = vec![
        "run".to_string(),
        "--name".to_string(),
        container_name.to_string(),
        "-u".to_string(),
        "foc-user".to_string(),
        "--network".to_string(),
        "host".to_string(),
        BUILDER_DOCKER_IMAGE.to_string(),
        "bash".to_string(),
        "-c".to_string(),
        cast_cmd,
    ];

    let key = "pdp_get_all_provider_ids";
    let output = run_and_log_command_strings("docker", &args, context, key)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to get provider IDs: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_provider_ids(&stdout)
}

/// Parse provider IDs from cast output.
/// Expected format: "[123,456,789]" or multi-line array format
fn parse_provider_ids(output: &str) -> Result<Vec<u64>, Box<dyn Error>> {
    let trimmed = output.trim();

    // Remove brackets and split by comma
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();

    if inner.is_empty() {
        return Ok(Vec::new());
    }

    inner
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<u64>()
                .map_err(|e| format!("Failed to parse provider ID '{}': {}", s, e).into())
        })
        .collect()
}
