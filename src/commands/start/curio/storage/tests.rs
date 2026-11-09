//! Regression coverage for preserving storage identity across repeated setup.

use std::fs;
use std::path::Path;

use super::{
    initialize_storage, CURIO_FAST_STORAGE_PATH, CURIO_LONG_TERM_STORAGE_PATH, CURIO_REPO_DIR,
    STORAGE_CONFIG_FILE, STORAGE_META_FILE,
};

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

/// Repeated setup must retain extra registrations without duplicating default paths.
#[test]
fn preserves_additional_registered_paths() {
    let volume = tempfile::tempdir().unwrap();
    let original = registered_paths_fixture();
    let path = seed_registration(volume.path(), &serde_json::to_vec(&original).unwrap());
    initialize_storage(volume.path()).unwrap();
    initialize_storage(volume.path()).unwrap();
    let mut result: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let paths = result["StoragePaths"].as_array_mut().unwrap();
    assert_eq!(
        paths
            .iter()
            .filter(|entry| entry["Path"] == CURIO_FAST_STORAGE_PATH)
            .count(),
        1
    );
    paths.retain(|entry| entry["Path"] != CURIO_FAST_STORAGE_PATH);
    assert_eq!(result, original);
}

/// Invalid registration must fail setup without destroying the original file.
#[test]
fn preserves_invalid_registration_on_error() {
    let volume = tempfile::tempdir().unwrap();
    let original = b"{invalid json";
    let path = seed_registration(volume.path(), original);
    assert!(initialize_storage(volume.path()).is_err());
    assert_eq!(fs::read(path).unwrap(), original);
}

/// Model an operator's extra path and policy, alongside one already registered default.
fn registered_paths_fixture() -> serde_json::Value {
    serde_json::json!({
        "StoragePaths": [
            {"Path": "/home/foc-user/curio/long-term-storage/extra", "Policy": "preserve"},
            {"Path": CURIO_LONG_TERM_STORAGE_PATH}
        ],
        "UnknownField": "preserve"
    })
}

/// Write an existing registration before offline initialization runs.
fn seed_registration(volume: &Path, bytes: &[u8]) -> std::path::PathBuf {
    let repo = volume.join(CURIO_REPO_DIR);
    fs::create_dir_all(&repo).unwrap();
    let path = repo.join(STORAGE_CONFIG_FILE);
    fs::write(&path, bytes).unwrap();
    path
}
