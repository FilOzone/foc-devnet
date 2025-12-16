//! PDP Service Provider registration step.
//!
//! This module handles registering the PDP_SP_0 account as a service provider
//! in the ServiceProviderRegistry contract and adding it to the approved
//! providers list in the FilecoinWarmStorageService contract.

mod constants;
mod pdp_service_provider_step;
mod provider_id;
mod registration;

pub use pdp_service_provider_step::PdpSpRegistrationStep;
