//! USER_0 deposit and operator approval step.
//!
//! This module handles setting up USER_0 for deal making by:
//! 1. Depositing USDFC tokens into FilecoinPay contract
//! 2. Approving WarmStorage as an operator with rate and lockup limits

pub mod constants;
pub mod operations;
pub mod user_deposit_permit_step;

pub use user_deposit_permit_step::UserDepositPermitStep;
