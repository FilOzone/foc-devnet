//! Contract addresses management for FOC deployment.
//!
//! This module handles loading, saving, and managing contract addresses
//! that are deployed during the FOC deployment process.
//!
//! All writes go through [`ContractAddresses::update`], which serializes
//! read-modify-write cycles behind a process-wide lock and replaces the file
//! atomically. Parallel startup steps (MockUSDFC, Multicall3, FOC deployments)
//! therefore cannot lose each other's entries, and concurrent readers never
//! observe a truncated file.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use crate::paths::contract_addresses_file;

/// Serializes read-modify-write cycles on the contract addresses file.
///
/// Steps execute as threads within a single process, so a process-wide mutex
/// is enough to prevent lost updates. Cross-process readers are still protected
/// by the atomic rename in [`address_write_atomic`].
static FILE_LOCK: Mutex<()> = Mutex::new(());

/// Contract addresses and deployment information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContractAddresses {
    /// Standard contracts (multicall, USDFC, etc.)
    #[serde(default)]
    pub contracts: std::collections::HashMap<String, String>,
    /// FOC service contracts
    #[serde(default)]
    pub foc_contracts: std::collections::HashMap<String, String>,
    /// FilBeam controller address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filbeam_controller: Option<String>,
    /// FilBeam beneficiary address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filbeam_beneficiary: Option<String>,
}

impl ContractAddresses {
    /// Load contract addresses from the state file
    pub fn load(run_id: &str) -> Result<Self, Box<dyn Error>> {
        Self::load_from(&contract_addresses_file(run_id))
    }

    /// Load contract addresses from an explicit path
    fn load_from(path: &Path) -> Result<Self, Box<dyn Error>> {
        if !path.exists() {
            return Err("Contract addresses file not found".into());
        }
        let content = fs::read_to_string(path)?;
        let addresses: ContractAddresses = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid contract addresses file {}: {}", path.display(), e))?;
        Ok(addresses)
    }

    /// Atomically apply a change to the contract addresses file.
    ///
    /// The whole read-modify-write cycle holds [`FILE_LOCK`], so concurrent
    /// updates merge instead of overwriting each other. A missing file starts
    /// from defaults; an unreadable or corrupt file is an error rather than
    /// silently replaced, so a bad write cannot erase already-deployed
    /// addresses.
    pub fn update<F>(run_id: &str, apply: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce(&mut ContractAddresses),
    {
        Self::update_file(&contract_addresses_file(run_id), apply)
    }

    fn update_file<F>(path: &Path, apply: F) -> Result<(), Box<dyn Error>>
    where
        F: FnOnce(&mut ContractAddresses),
    {
        let _guard = FILE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let mut addresses = if path.exists() {
            Self::load_from(path)?
        } else {
            Self::default()
        };

        apply(&mut addresses);

        address_write_atomic(path, &addresses)
    }
}

/// Write `addresses` to `path` by renaming a fully written temporary file.
///
/// Rename within the same directory is atomic on POSIX, so readers see either
/// the previous complete file or the new complete file — never a partial write.
fn address_write_atomic(path: &Path, addresses: &ContractAddresses) -> Result<(), Box<dyn Error>> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("Contract addresses file has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)?;

    let json = serde_json::to_vec_pretty(addresses)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(&json)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)
        .map_err(|e| format!("Failed to persist {}: {}", path.display(), e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn sample_address(index: usize) -> String {
        format!("0x{:040x}", index)
    }

    #[test]
    fn test_concurrent_updates_preserve_all_entries() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("contract_addresses.json");
        let thread_count = 8;
        let barrier = Arc::new(Barrier::new(thread_count));

        let handles: Vec<_> = (0..thread_count)
            .map(|i| {
                let path = path.clone();
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    ContractAddresses::update_file(&path, |addresses| {
                        addresses
                            .contracts
                            .insert(format!("contract_{}", i), sample_address(i));
                    })
                    .unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let addresses = ContractAddresses::load_from(&path).unwrap();
        assert_eq!(addresses.contracts.len(), thread_count);
        for i in 0..thread_count {
            assert_eq!(
                addresses.contracts.get(&format!("contract_{}", i)),
                Some(&sample_address(i))
            );
        }
    }

    #[test]
    fn test_update_merges_sections_without_dropping_existing_fields() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("contract_addresses.json");

        ContractAddresses::update_file(&path, |addresses| {
            addresses
                .contracts
                .insert("usdfc".to_string(), sample_address(1));
        })
        .unwrap();

        ContractAddresses::update_file(&path, |addresses| {
            addresses.foc_contracts.insert(
                "service_provider_registry_proxy".to_string(),
                sample_address(2),
            );
            addresses.filbeam_controller = Some(sample_address(3));
        })
        .unwrap();

        ContractAddresses::update_file(&path, |addresses| {
            addresses
                .contracts
                .insert("multicall".to_string(), sample_address(4));
        })
        .unwrap();

        let addresses = ContractAddresses::load_from(&path).unwrap();
        assert_eq!(addresses.contracts.len(), 2);
        assert_eq!(addresses.foc_contracts.len(), 1);
        assert_eq!(
            addresses.filbeam_controller.as_deref(),
            Some(sample_address(3).as_str())
        );
    }

    #[test]
    fn test_readers_never_observe_partial_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("contract_addresses.json");
        let path = Arc::new(path);

        ContractAddresses::update_file(&path, |addresses| {
            addresses
                .contracts
                .insert("seed".to_string(), sample_address(0));
        })
        .unwrap();

        let writer_path = Arc::clone(&path);
        let writer = thread::spawn(move || {
            for i in 1..=300 {
                ContractAddresses::update_file(&writer_path, |addresses| {
                    addresses
                        .contracts
                        .insert(format!("rolling_{}", i), sample_address(i));
                    // Grow and shrink the file so a non-atomic write would
                    // frequently be observed mid-truncation.
                    if i % 2 == 0 {
                        addresses.contracts.clear();
                    }
                })
                .unwrap();
            }
        });

        for _ in 0..600 {
            let content = fs::read_to_string(&*path).unwrap();
            serde_json::from_str::<ContractAddresses>(&content)
                .expect("reader observed a partial or invalid contract addresses file");
        }

        writer.join().unwrap();
    }

    #[test]
    fn test_update_refuses_to_overwrite_corrupt_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("contract_addresses.json");
        fs::write(&path, "{ not json").unwrap();

        let result = ContractAddresses::update_file(&path, |addresses| {
            addresses
                .contracts
                .insert("usdfc".to_string(), sample_address(1));
        });

        assert!(result.is_err());
        // The corrupt content must be left untouched rather than replaced.
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
    }
}
