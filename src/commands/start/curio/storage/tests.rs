//! Regression coverage for preserving storage identity across repeated setup.

use std::fs;
use std::path::Path;

use super::{initialize_storage, STORAGE_META_FILE};

/// Reinitialization must not orphan pieces by replacing storage IDs or resetting policy.
#[test]
fn repeated_setup_preserves_existing_storage() {
    let volume = tempfile::tempdir().unwrap();
    let store = seed_existing_piece(volume.path());
    let before = storage_metadata(volume.path());
    initialize_storage(volume.path()).unwrap();
    assert_eq!(storage_metadata(volume.path()), before);
    assert_eq!(
        fs::read(store.join("existing-piece")).unwrap(),
        b"piece payload"
    );
}

/// Add a piece and storage policy that setup must leave untouched.
fn seed_existing_piece(volume: &Path) -> std::path::PathBuf {
    initialize_storage(volume).unwrap();
    let store = volume.join("long-term-storage");
    let metadata_path = store.join(STORAGE_META_FILE);
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).unwrap()).unwrap();
    metadata["DenyTypes"] = serde_json::json!(["cache"]);
    fs::write(&metadata_path, serde_json::to_vec(&metadata).unwrap()).unwrap();
    fs::write(store.join("existing-piece"), b"piece payload").unwrap();
    store
}

/// Capture both storage metadata files byte-for-byte, including unknown fields.
fn storage_metadata(volume: &Path) -> Vec<Vec<u8>> {
    ["fast-storage", "long-term-storage"]
        .iter()
        .map(|directory| fs::read(volume.join(directory).join(STORAGE_META_FILE)).unwrap())
        .collect()
}
