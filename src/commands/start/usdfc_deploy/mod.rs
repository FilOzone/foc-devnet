//! MockUSDFC Token Deployment step.
//!
//! This module handles the deployment of the MockUSDFC ERC-20 token
//! using the Foundry project located in contracts/MockUSDFC/.
//!
//! The deployment is delegated to the Foundry scripts which handle:
//! - Contract compilation with OpenZeppelin dependencies
//! - Deployment via forge script
//! - Verification of deployed contract functions
//!
//! This approach provides better separation of concerns and easier debugging.

pub mod contract_storage;
pub mod deployment;
pub mod foundry_setup;
pub mod key_management;
pub mod prerequisites;
pub mod usdfc_deploy_step;
pub mod verification;

pub use usdfc_deploy_step::USDFCDeployStep;
