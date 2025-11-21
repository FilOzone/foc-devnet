//! Genesis file construction and configuration.
//!
//! This module handles creating and modifying the genesis JSON file
//! with signers, miners, and pre-funded accounts.

use crate::commands::start::genesis::keys::get_bls_addresses;
use crate::paths::{
    foc_localnet_bin, foc_localnet_docker_volumes, foc_localnet_genesis,
    foc_localnet_genesis_sectors, foc_localnet_lotus_keys,
};
use crossterm::style::Stylize;
use std::fs;

/// Construct the complete genesis configuration.
///
/// This combines the genesis construction steps:
/// 1. Create initial genesis file
/// 2. Add signers
/// 3. Add miner
/// 4. Add pre-funded accounts (if any)
///
/// If the genesis file already exists, all construction steps are skipped
/// since the genesis configuration is considered complete.
pub fn construct_genesis() -> Result<(), Box<dyn std::error::Error>> {
    let genesis_dir = foc_localnet_genesis();
    let genesis_file_path = genesis_dir.join(super::constants::GENESIS_FILE);

    // Check if genesis file already exists - if so, skip all construction
    if genesis_file_path.exists() {
        println!(
            "  {} Genesis file already exists at {}",
            "✓".green(),
            genesis_file_path.display()
        );
        println!("{}", "✓ Genesis construction complete".green().bold());
        return Ok(());
    }

    println!("{}", "Constructing genesis configuration...".blue().bold());

    create_genesis_file()?;
    add_signers_to_genesis()?;
    add_miner_to_genesis()?;
    add_prefunded_accounts()?;

    println!("{}", "✓ Genesis construction complete".green().bold());
    Ok(())
}

/// Create the initial genesis file.
///
/// Runs `lotus-seed genesis new` to create a new genesis file with the network name
/// and current timestamp.
///
/// Note: This function assumes the genesis file does not already exist.
/// The caller (`construct_genesis`) should check for existence first.
fn create_genesis_file() -> Result<(), Box<dyn std::error::Error>> {
    let genesis_dir = foc_localnet_genesis();

    println!("  {} Creating genesis file...", "📜".cyan());

    // Ensure genesis directory exists
    fs::create_dir_all(&genesis_dir)?;

    // Get current timestamp in ISO 8601 format (RFC3339)
    // lotus-seed expects format like: 2006-01-02T15:04:05Z
    let now = std::time::SystemTime::now();
    let datetime: chrono::DateTime<chrono::Utc> = now.into();
    let timestamp = datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Run lotus-seed genesis new in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/opt/bin", bin_dir.display()),
            "-v",
            &format!("{}:/root/.cargo", builder_volumes_dir.join("cargo").display()),
            "-v",
            &format!("{}:/genesis", genesis_dir.display()),
            "foc-builder",
            "/bin/bash",
            "-c",
            &format!(
                "/opt/bin/lotus-seed genesis new --network-name {} --timestamp {} /genesis/{} && chmod 666 /genesis/{}",
                super::constants::NETWORK_NAME, timestamp, super::constants::GENESIS_FILE, super::constants::GENESIS_FILE
            ),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create genesis file: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Genesis file created successfully", "✓".green());
    Ok(())
}

/// Add signers to the genesis file.
///
/// Runs `lotus-seed genesis set-signers` to add the BLS signer keys
/// with the configured threshold.
fn add_signers_to_genesis() -> Result<(), Box<dyn std::error::Error>> {
    println!("  {} Adding signers to genesis...", "🔑".cyan());

    let genesis_dir = foc_localnet_genesis();
    let keys_dir = foc_localnet_lotus_keys();

    // Get signer BLS addresses
    let addresses = get_bls_addresses("key", super::constants::NUM_SIGNER_KEYS)?;

    if addresses.len() != super::constants::NUM_SIGNER_KEYS as usize {
        return Err(format!(
            "Expected {} BLS signer addresses, found {}",
            super::constants::NUM_SIGNER_KEYS,
            addresses.len()
        )
        .into());
    }
    println!("    Using signers:");
    for (i, addr) in addresses.iter().enumerate() {
        println!("      {} {}", format!("Key {}:", i + 1).dim(), addr);
    }

    // Run lotus-seed genesis set-signers in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/opt/bin", bin_dir.display()),
            "-v",
            &format!("{}:/root/.cargo", builder_volumes_dir.join("cargo").display()),
            "-v",
            &format!("{}:/genesis", genesis_dir.display()),
            "-v",
            &format!("{}:/keys", keys_dir.display()),
            "foc-builder",
            "/bin/bash",
            "-c",
            &format!(
                "/opt/bin/lotus-seed genesis set-signers --threshold={} --signers {} --signers {} /genesis/{}",
                super::constants::SIGNERS_THRESHOLD, addresses[0], addresses[1], super::constants::GENESIS_FILE
            ),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to add signers to genesis: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Signers added successfully", "✓".green());
    Ok(())
}

/// Add a miner to the genesis file.
///
/// Runs `lotus-seed genesis add-miner` to add the pre-sealed miner (t01000)
/// to the genesis configuration.
fn add_miner_to_genesis() -> Result<(), Box<dyn std::error::Error>> {
    println!("  {} Adding miner to genesis...", "⛏".cyan());

    let genesis_dir = foc_localnet_genesis();
    let sectors_dir = foc_localnet_genesis_sectors();

    // Check for pre-seal file (typically pre-seal-t01000.json)
    let preseal_file = sectors_dir.join("pre-seal-t01000.json");

    if !preseal_file.exists() {
        return Err(format!(
            "Pre-seal file not found at {}. Ensure sectors are pre-sealed first.",
            preseal_file.display()
        )
        .into());
    }

    // Run lotus-seed genesis add-miner in builder container
    let bin_dir = foc_localnet_bin();
    let builder_volumes_dir = foc_localnet_docker_volumes().join("builder");

    let output = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/opt/bin", bin_dir.display()),
            "-v",
            &format!("{}:/root/.cargo", builder_volumes_dir.join("cargo").display()),
            "-v",
            &format!("{}:/genesis", genesis_dir.display()),
            "-v",
            &format!("{}:/root/.genesis-sectors", sectors_dir.display()),
            "foc-builder",
            "/bin/bash",
            "-c",
            &format!(
                "/opt/bin/lotus-seed genesis add-miner /genesis/{} /root/.genesis-sectors/pre-seal-t01000.json",
                super::constants::GENESIS_FILE
            ),
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to add miner to genesis: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    println!("  {} Miner added to genesis successfully", "✓".green());
    Ok(())
}

/// Add pre-funded accounts to the genesis file.
///
/// Since lotus-seed doesn't have an `add-actor` command, we modify the genesis JSON
/// directly to add additional pre-funded accounts that are not signers.
fn add_prefunded_accounts() -> Result<(), Box<dyn std::error::Error>> {
    if super::constants::NUM_PREFUNDED_KEYS == 0 {
        return Ok(());
    }

    println!("  {} Adding pre-funded accounts to genesis...", "💰".cyan());

    let genesis_dir = foc_localnet_genesis();
    let genesis_file_path = genesis_dir.join(super::constants::GENESIS_FILE);

    // Get pre-funded BLS addresses
    let addresses = get_bls_addresses("prefunded", super::constants::NUM_PREFUNDED_KEYS)?; // Read the genesis file
    let genesis_content = fs::read_to_string(&genesis_file_path)?;
    let mut genesis: serde_json::Value = serde_json::from_str(&genesis_content)?;

    // Add each pre-funded account to the Accounts array
    if let Some(accounts) = genesis.get_mut("Accounts").and_then(|v| v.as_array_mut()) {
        for (i, addr) in addresses.iter().enumerate() {
            // Create account entry with testnet format (t3...)
            let account = serde_json::json!({
                "Type": "account",
                "Balance": "50000000000000000000000",  // 50,000 FIL
                "Meta": {
                    "Owner": format!("t{}", &addr[1..])  // Convert f3... to t3...
                }
            });

            accounts.push(account);
            println!(
                "      {} Pre-funded account {}: {}",
                "✓".green(),
                i + 1,
                addr
            );
        }
    } else {
        return Err("Genesis file does not have an 'Accounts' array".into());
    }

    // Write the modified genesis back
    let updated_content = serde_json::to_string_pretty(&genesis)?;
    fs::write(&genesis_file_path, updated_content)?;

    println!("  {} Pre-funded accounts added successfully", "✓".green());
    Ok(())
}
