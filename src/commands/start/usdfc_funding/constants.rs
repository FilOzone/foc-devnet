//! Constants and allocation planning for MockUSDFC token distribution.
//!
//! This module defines constants used in the MockUSDFC funding process.

use crate::constants::{
    mock_usdfc_sp_allocation_tokens, MOCK_USDFC_USER_ALLOCATION_TOKENS, USER_ACCOUNT_COUNT,
};
use ethers_core::types::U256;

/// Convert token amount to wei (multiply by 10^18)
pub fn token_amount_to_wei(tokens: u64) -> U256 {
    U256::from(tokens) * U256::exp10(18)
}

/// Build the startup allocation plan as `(account name, whole tokens)` pairs.
///
/// Balance checks, transfers, and post-transfer verification all derive from
/// this plan so they cannot drift apart. The plan never exceeds the MockUSDFC
/// initial supply; see [`mock_usdfc_sp_allocation_tokens`].
pub fn distribution_plan(active_pdp_sp_count: usize) -> Vec<(String, u64)> {
    let mut plan = Vec::with_capacity(USER_ACCOUNT_COUNT + active_pdp_sp_count);

    for user_num in 1..=USER_ACCOUNT_COUNT {
        plan.push((
            format!("USER_{}", user_num),
            MOCK_USDFC_USER_ALLOCATION_TOKENS,
        ));
    }

    let sp_amount = mock_usdfc_sp_allocation_tokens(active_pdp_sp_count);
    for sp_num in 1..=active_pdp_sp_count {
        plan.push((format!("PDP_SP_{}", sp_num), sp_amount));
    }

    plan
}

/// Transaction confirmation wait time in seconds
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 5;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        mock_usdfc_distribution_total_tokens, MAX_PDP_SP_COUNT, MOCK_USDFC_INITIAL_SUPPLY_TOKENS,
        MOCK_USDFC_SP_ALLOCATION_TOKENS,
    };

    #[test]
    fn plan_lists_every_user_and_active_sp() {
        let plan = distribution_plan(2);
        let names: Vec<&str> = plan.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            ["USER_1", "USER_2", "USER_3", "PDP_SP_1", "PDP_SP_2"]
        );
    }

    #[test]
    fn plan_ignores_inactive_sps() {
        let plan = distribution_plan(0);
        assert_eq!(plan.len(), USER_ACCOUNT_COUNT);
        assert!(plan.iter().all(|(name, _)| name.starts_with("USER_")));
    }

    #[test]
    fn plan_keeps_default_allocations() {
        let plan = distribution_plan(2);
        assert_eq!(plan[0].1, 100_000);
        assert_eq!(plan[3].1, MOCK_USDFC_SP_ALLOCATION_TOKENS);
    }

    #[test]
    fn plan_fits_within_supply_at_max_sp_count() {
        let plan = distribution_plan(MAX_PDP_SP_COUNT);
        let total: u64 = plan.iter().map(|(_, tokens)| tokens).sum();
        assert!(total <= MOCK_USDFC_INITIAL_SUPPLY_TOKENS);
        assert_eq!(
            total,
            mock_usdfc_distribution_total_tokens(MAX_PDP_SP_COUNT)
        );
    }
}
