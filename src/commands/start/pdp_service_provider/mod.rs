//! PDP Service Provider registration step.
//!
//! This module handles registering multiple PDP service provider accounts
//! in the ServiceProviderRegistry contract and adding approved ones to the
//! approved providers list in the FilecoinWarmStorageService contract.

mod constants;
mod pdp_service_provider_step;
mod provider_id;
mod registration;

pub use pdp_service_provider_step::PdpSpRegistrationStep;
