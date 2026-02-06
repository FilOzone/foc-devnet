//! Endorsement step for PDP service providers.
//!
//! This module handles endorsing approved PDP service providers in the
//! endorsements contract (ProviderIdSet). Endorsed providers are a privileged
//! subset of approved providers that meet quality and reliability standards.

mod constants;
mod endorsement_step;
mod operations;

pub use endorsement_step::EndorsementStep;
