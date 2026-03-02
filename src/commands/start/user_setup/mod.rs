//! USER_1 payment setup step.
//!
//! Configures USER_1's wallet for FOC storage services after contracts are
//! deployed, using on-chain cast transactions rather than JS scripts.

mod constants;
mod operations;
mod user_setup_step;

pub use user_setup_step::UserSetupStep;
