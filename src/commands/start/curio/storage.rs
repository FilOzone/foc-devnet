//! Initialize Curio storage files before the daemon discovers its hashspace drives.

use std::error::Error;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::Path;

use serde_json::{json, Value};

use super::constants::{CURIO_FAST_STORAGE_PATH, CURIO_LONG_TERM_STORAGE_PATH};

const CURIO_REPO_DIR: &str = ".curio";
const STORAGE_CONFIG_FILE: &str = "storage.json";
const STORAGE_META_FILE: &str = "sectorstore.json";
const STORAGE_WEIGHT: u64 = 10;

#[cfg(test)]
mod tests;

/// Register both container paths before Curio initializes the PDP hashspace.
pub(super) fn initialize_storage(volume: &Path) -> Result<(), Box<dyn Error>> {
    let repo = volume.join(CURIO_REPO_DIR);
    fs::create_dir_all(&repo)?;
    initialize_path(volume, CURIO_FAST_STORAGE_PATH, true)?;
    initialize_path(volume, CURIO_LONG_TERM_STORAGE_PATH, false)?;
    let config = json!({"StoragePaths": [
        {"Path": CURIO_FAST_STORAGE_PATH}, {"Path": CURIO_LONG_TERM_STORAGE_PATH}
    ]});
    write_json(&repo.join(STORAGE_CONFIG_FILE), &config)
}

/// Create metadata only for new paths, preserving identities and policy on subsequent starts.
fn initialize_path(volume: &Path, container_path: &str, seal: bool) -> Result<(), Box<dyn Error>> {
    let directory = volume.join(
        Path::new(container_path)
            .file_name()
            .ok_or("Missing storage path name")?,
    );
    fs::create_dir_all(&directory)?;
    let path = directory.join(STORAGE_META_FILE);
    match File::create_new(&path) {
        Ok(file) => serde_json::to_writer_pretty(file, &storage_metadata(seal)).map_err(Into::into),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(format!(
            "Creating Curio storage metadata {}: {}",
            path.display(),
            error
        )
        .into()),
    }
}

/// Generate the same storage capabilities as `storage attach --init --seal/--store`.
fn storage_metadata(seal: bool) -> Value {
    json!({
        "ID": format!("{:032x}", rand::random::<u128>()),
        "Weight": STORAGE_WEIGHT,
        "CanSeal": seal,
        "CanStore": !seal,
        "MaxStorage": 0
    })
}

/// Persist container-visible storage configuration with file context on failure.
fn write_json(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes).map_err(|error| {
        format!(
            "Writing Curio storage configuration {}: {}",
            path.display(),
            error
        )
        .into()
    })
}
