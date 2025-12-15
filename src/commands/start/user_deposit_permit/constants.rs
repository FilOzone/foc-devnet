//! Constants for USER_0 deposit and operator approval.

/// Account name for the user
pub const USER_ACCOUNT: &str = "USER_0";

/// USDFC deposit amount in tokens (not wei)
/// This is the amount USER_0 will deposit into FilecoinPay
pub const DEPOSIT_AMOUNT_TOKENS: u64 = 1_000;

/// Lockup allowance in seconds (30 days)
/// This is the maximum lockup period WarmStorage can lock funds for
pub const LOCKUP_ALLOWANCE_SECONDS: u64 = 30 * 24 * 60 * 60; // 30 days

/// Rate allowance (maximum uint256 for unlimited rate)
/// This is the maximum rate WarmStorage can charge
pub const RATE_ALLOWANCE: &str =
    "115792089237316195423570985008687907853269984665640564039457584007913129639935"; // max uint256

/// Max allowance (maximum uint256 for unlimited operations)
/// This is the maximum number of operations WarmStorage can perform
pub const MAX_ALLOWANCE: &str =
    "115792089237316195423570985008687907853269984665640564039457584007913129639935"; // max uint256

/// Transaction confirmation wait time in seconds
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 8;
