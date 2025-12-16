//! Contract interaction operations for USER_0 deposit and permit.

mod approvals;
mod deposits;
mod operators;
mod utils;

pub use approvals::{approve_usdfc_for_filecoin_pay, query_usdfc_allowance};
pub use deposits::{deposit_usdfc_to_filecoin_pay, query_filecoin_pay_balance};
pub use operators::{query_operator_allowance, set_operator_approval};
pub use utils::token_amount_to_wei;
