//! FOC deployment module.
//!
//! Deploys Filecoin Onchain Cloud (FOC) contracts for local devnets.

pub mod contract_addresses;
pub mod deployment;
pub mod foc_deploy_step;
pub mod helpers;

// Re-export the main step struct for convenience
pub use foc_deploy_step::FOCDeployStep;
