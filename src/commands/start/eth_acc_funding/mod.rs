//! Ethereum Account Funding step.
//!
//! This module handles the creation and funding of Ethereum-compatible accounts
//! required for FOC contract deployment. It creates f4 (delegated) addresses
//! and funds them with FIL for FEVM operations.

pub mod constants;
pub mod lotus_checks;
pub mod key_operations;
pub mod funding_operations;
pub mod eth_acc_funding_step;

pub use eth_acc_funding_step::ETHAccFundingStep;