//! MockUSDFC token distribution step.
//!
//! This module handles distributing MockUSDFC tokens to user and service provider addresses,
//! similar to how FIL is distributed in the ETHAccFundingStep.

pub mod constants;
pub mod usdfc_funding_step;
pub mod funding_operations;
pub mod key_operations;

pub use usdfc_funding_step::USDFCFundingStep;