//! Proof parameters status display for foc-devnet.
//!
//! This module displays the proof parameters docker volume status,
//! including availability.

use crate::paths::foc_devnet_proof_parameters;
use tracing::info;

/// Print the proof parameters status.
///
/// Displays whether the proof parameters docker volume is available.
pub fn print_proof_params_status() -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = foc_devnet_proof_parameters();

    if !params_dir.exists() {
        info!("Run 'foc-devnet start' to download proof parameters. (first run downloads and caches it for future runs)");
        return Ok(());
    }

    // Check if the directory has content
    if params_dir.read_dir()?.next().is_some() {
        info!("FilProofParams: OK, at: {}", params_dir.display());
    } else {
        info!("FilProofParams: EMPTY, at: {}", params_dir.display());
        info!("Run 'foc-devnet start' to download proof parameters.");
    }

    Ok(())
}
