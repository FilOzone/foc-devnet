//! Provider registration contract interactions.

use super::constants::*;
use crate::constants::BUILDER_CONTAINER;
use crossterm::style::Stylize;
use std::error::Error;
use std::process::Command;

/// Register provider in ServiceProviderRegistry contract
///
/// Returns the provider ID assigned by the registry.
pub fn register_provider(
    run_id: &str,
    registry_address: &str,
    pdp_sp_0_address: &str,
    pdp_sp_0_eth_address: &str,
    mock_usdfc_address: &str,
    lotus_rpc_url: &str,
) -> Result<u64, Box<dyn Error>> {
    let _ = run_id; // Not needed when using foc-builder

    println!("  Registering PDP_SP_0 in ServiceProviderRegistry...");

    // Get private key for PDP_SP_0
    let pdp_sp_0_private_key =
        crate::commands::start::foc_deployer::get_private_key(pdp_sp_0_address, "")?;

    // Build capability keys array
    let cap_keys = build_capability_keys();

    // Build capability values array
    let cap_values = build_capability_values(mock_usdfc_address)?;

    // Calculate registration fee in wei
    let registration_fee_wei = format!("{}000000000000000000", REGISTRATION_FEE_FIL);

    // Execute registerProvider transaction with high gas limit for FEVM
    // FEVM consistently requires much higher gas than Ethereum, so we use a large fixed limit
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "bash",
            "-c",
            &format!(
                r#"cast send {} \
                "registerProvider(address,string,string,uint8,string[],bytes[])" \
                {} \
                "{}" \
                "{}" \
                0 \
                {} \
                {} \
                --value {} \
                --rpc-url {} \
                --private-key {} \
                --gas-limit 10000000000"#,
                registry_address,
                pdp_sp_0_eth_address,
                PROVIDER_NAME,
                PROVIDER_DESCRIPTION,
                cap_keys,
                cap_values,
                registration_fee_wei,
                lotus_rpc_url,
                pdp_sp_0_private_key
            ),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to register provider: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // println!("  Registration output:\n{}", stdout);

    // Check if transaction was successful by looking for "status" field
    if stdout.contains("status               0") || stdout.contains("status               (failed)")
    {
        return Err("Provider registration transaction failed (status 0). Check transaction logs for details.".into());
    }

    // Wait for transaction confirmation
    wait_for_confirmation();

    // Query provider ID
    let provider_id = query_provider_id(registry_address, pdp_sp_0_eth_address, lotus_rpc_url)?;

    println!("  {} Provider ID: {}", "✓".green(), provider_id);
    Ok(provider_id)
}

/// Add provider to approved list in WarmStorage contract
pub fn add_to_approved_list(
    run_id: &str,
    warm_storage_address: &str,
    provider_id: u64,
    deployer_foc_address: &str,
    _deployer_foc_eth_address: &str,
    lotus_rpc_url: &str,
) -> Result<(), Box<dyn Error>> {
    let _ = run_id; // Not needed when using foc-builder

    println!(
        "  Adding provider {} to WarmStorage approved list...",
        provider_id
    );

    // Get private key for DEPLOYER_FOC
    let deployer_foc_private_key =
        crate::commands::start::foc_deployer::get_private_key(deployer_foc_address, "")?;

    // Use high gas limit for FEVM (cast send doesn't support gas-estimate-multiplier)
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "cast",
            "send",
            warm_storage_address,
            "addApprovedProvider(uint256)",
            &provider_id.to_string(),
            "--rpc-url",
            lotus_rpc_url,
            "--private-key",
            &deployer_foc_private_key,
            "--gas-limit",
            "10000000000",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to add approved provider: {}", stderr).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check if transaction was successful
    if stdout.contains("status               0") || stdout.contains("status               (failed)")
    {
        println!("  Transaction output:\n{}", stdout);
        return Err("Add approved provider transaction failed (status 0). Check transaction logs for details.".into());
    }

    println!("  {} Provider added to approved list", "✓".green());
    wait_for_confirmation();

    Ok(())
}

/// Build capability keys array for cast (no quotes, bracket format)
fn build_capability_keys() -> String {
    "[serviceURL,minPieceSizeInBytes,maxPieceSizeInBytes,storagePricePerTibPerDay,minProvingPeriodInEpochs,location,paymentTokenAddress]".to_string()
}

/// Build capability values array with properly ABI-encoded bytes values (no quotes, bracket format)
fn build_capability_values(mock_usdfc_address: &str) -> Result<String, Box<dyn Error>> {
    // For the bytes[] parameter in Solidity, we need to pass raw bytes for each value
    // Cast expects array format: [0x...,0x...,0x...] (no quotes, no spaces)

    // Encode each value using big-endian minimal encoding (like BigEndian.sol does)
    let service_url_bytes = hex::encode(DEFAULT_SERVICE_URL.as_bytes());
    let location_bytes = hex::encode(LOCATION.as_bytes());

    // For uint256 values, encode as minimal big-endian bytes (no leading zeros)
    let min_piece_size_bytes = encode_uint_minimal(MIN_PIECE_SIZE_BYTES);
    let max_piece_size_bytes = encode_uint_minimal(MAX_PIECE_SIZE_BYTES);
    let storage_price_bytes = encode_uint_minimal(STORAGE_PRICE_PER_TIB_PER_DAY);
    let min_proving_period_bytes = encode_uint_minimal(MIN_PROVING_PERIOD_EPOCHS);

    // Payment token address - just the address bytes (20 bytes)
    let payment_token_bytes = &mock_usdfc_address[2..]; // Remove 0x prefix, will add back

    // Build the array - cast expects format: [0x...,0x...,0x...] (no quotes, no spaces)
    let values = format!(
        "[0x{},0x{},0x{},0x{},0x{},0x{},0x{}]",
        service_url_bytes,
        min_piece_size_bytes,
        max_piece_size_bytes,
        storage_price_bytes,
        min_proving_period_bytes,
        location_bytes,
        payment_token_bytes
    );
    Ok(values)
}

/// Encode a uint64 as minimal big-endian hex (no leading zeros)
fn encode_uint_minimal(value: u64) -> String {
    if value == 0 {
        return "00".to_string();
    }

    // Convert to big-endian bytes
    let bytes = value.to_be_bytes();

    // Skip leading zeros
    let first_non_zero = bytes
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(bytes.len() - 1);

    // Encode remaining bytes as hex
    hex::encode(&bytes[first_non_zero..])
}

/// Query provider ID from registry
fn query_provider_id(
    registry_address: &str,
    pdp_sp_0_eth_address: &str,
    lotus_rpc_url: &str,
) -> Result<u64, Box<dyn Error>> {
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "cast",
            "call",
            registry_address,
            "getProviderIdByAddress(address)(uint256)",
            pdp_sp_0_eth_address,
            "--rpc-url",
            lotus_rpc_url,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query provider ID: {}", stderr).into());
    }

    let result = String::from_utf8_lossy(&output.stdout);
    let provider_id: u64 = result.trim().parse().unwrap_or(0);

    if provider_id == 0 {
        return Err("Provider ID is 0, registration may have failed".into());
    }

    Ok(provider_id)
}

/// Wait for transaction confirmation
fn wait_for_confirmation() {
    println!(
        "  Waiting {} seconds for transaction confirmation...",
        TRANSACTION_CONFIRMATION_WAIT_SECS
    );
    std::thread::sleep(std::time::Duration::from_secs(
        TRANSACTION_CONFIRMATION_WAIT_SECS,
    ));
}

/// Verify provider count on-chain
///
/// Returns the total number of registered providers.
pub fn verify_provider_count(
    run_id: &str,
    registry_address: &str,
    lotus_rpc_url: &str,
) -> Result<u64, Box<dyn Error>> {
    let _ = run_id; // Not needed when using foc-builder

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "cast",
            "call",
            registry_address,
            "getProviderCount()(uint256)",
            "--rpc-url",
            lotus_rpc_url,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query provider count: {}", stderr).into());
    }

    let result = String::from_utf8_lossy(&output.stdout);
    let count: u64 = result.trim().parse().unwrap_or(0);

    Ok(count)
}

/// Verify provider ID by address on-chain
///
/// Returns the provider ID for the given address.
pub fn verify_provider_id_by_address(
    run_id: &str,
    registry_address: &str,
    provider_address: &str,
    lotus_rpc_url: &str,
) -> Result<u64, Box<dyn Error>> {
    let _ = run_id; // Not needed when using foc-builder

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "cast",
            "call",
            registry_address,
            "getProviderIdByAddress(address)(uint256)",
            provider_address,
            "--rpc-url",
            lotus_rpc_url,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query provider ID by address: {}", stderr).into());
    }

    let result = String::from_utf8_lossy(&output.stdout);
    let provider_id: u64 = result.trim().parse().unwrap_or(0);

    Ok(provider_id)
}

/// Verify provider is in approved list using StateView contract
///
/// Uses FilecoinWarmStorageServiceStateView for read-only queries.
/// Returns true if the provider is approved.
pub fn verify_approved_provider(
    run_id: &str,
    state_view_address: &str,
    provider_id: u64,
    lotus_rpc_url: &str,
) -> Result<bool, Box<dyn Error>> {
    let _ = run_id; // Not needed when using foc-builder

    // Use isProviderApproved function on StateView contract
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "host",
            BUILDER_CONTAINER,
            "cast",
            "call",
            state_view_address,
            "isProviderApproved(uint256)(bool)",
            &provider_id.to_string(),
            "--rpc-url",
            lotus_rpc_url,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to query if provider is approved: {}", stderr).into());
    }

    let result = String::from_utf8_lossy(&output.stdout);
    let is_approved = result.trim() == "true";

    Ok(is_approved)
}
