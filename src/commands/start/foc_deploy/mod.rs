//! FOC deployment module.
//!
//! This module contains all components related to deploying FOC (Filecoin Onchain Contracts)
//! including contract addresses management, deployment logic, and step execution.

pub mod contract_addresses;
pub mod deployment;
pub mod foc_deploy_step;
pub mod helpers;

// Re-export the main step struct for convenience
pub use foc_deploy_step::FOCDeployStep;
