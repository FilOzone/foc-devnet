//! Constants for PDP service provider registration.

/// Registration fee in FIL
pub const REGISTRATION_FEE_FIL: u64 = 5;

/// Transaction confirmation wait time in seconds
pub const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 8;

/// Minimum piece size in bytes (1 KiB)
pub const MIN_PIECE_SIZE_BYTES: u64 = 1024;

/// Maximum piece size in bytes (1 GiB)
pub const MAX_PIECE_SIZE_BYTES: u64 = 1024 * 1024 * 1024;

/// Storage price per TiB per day (1 FIL in attoFIL)
pub const STORAGE_PRICE_PER_TIB_PER_DAY: u64 = 1000000000000000000;

/// Minimum proving period in epochs (~1 day)
pub const MIN_PROVING_PERIOD_EPOCHS: u64 = 2880;

/// Geographic location identifier
pub const LOCATION: &str = "DevNet";

/// Provider description
pub const PROVIDER_DESCRIPTION: &str = "PDP Service Provider 0 for DevNet";
