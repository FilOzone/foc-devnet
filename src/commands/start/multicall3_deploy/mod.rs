//! Multicall3 Contract Deployment step.
//!
//! This module handles the deployment of the Multicall3 contract
//! from the mds1/multicall3 repository.
//!
//! The deployment uses Foundry to compile and deploy the contract
//! to the local Filecoin network.

pub mod contract_storage;
pub mod deployment;
pub mod key_management;
pub mod multicall3_deploy_step;
pub mod prerequisites;
pub mod verification;

pub use multicall3_deploy_step::MultiCall3DeployStep;
