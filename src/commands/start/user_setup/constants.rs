//! Constants for user payment setup operations.

/// Container name suffix for ERC20 approve transaction.
pub const CONTAINER_ERC20_APPROVE: &str = "user-erc20-approve";

/// Container name suffix for FilecoinPay deposit transaction.
pub const CONTAINER_FP_DEPOSIT: &str = "user-fp-deposit";

/// Container name suffix for FilecoinPay operator approval transaction.
pub const CONTAINER_FP_APPROVE_OPERATOR: &str = "user-fp-approve-operator";

/// Gas limit for cast send transactions on Filecoin FEVM.
pub const CAST_GAS_LIMIT: &str = "100000000";

/// 10 USDFC expressed in the token's 18-decimal base unit.
///
/// Covers cumulative lockups when CI scenarios share the pre-funded user.
pub const USDFC_DEPOSIT_AMOUNT: &str = "10000000000000000000";

/// uint256 max value, used for unlimited operator approval allowances.
pub const MAX_UINT256: &str =
    "115792089237316195423570985008687907853269984665640564039457584007913129639935";

/// 30-day lockup period expressed in Filecoin epochs (2880 epochs/day * 30 days).
pub const LOCKUP_PERIOD_EPOCHS: &str = "86400";

/// Seconds to wait after on-chain payment setup before completing the step,
/// allowing transactions to be included in a block.
pub const POST_SETUP_WAIT_SECONDS: u64 = 5;
