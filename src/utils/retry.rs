//! Retry utilities for validation operations.
//!
//! This module provides retry logic for validation and verification steps.
//! NEVER use these for transactions or deployments - only for read-only checks.

use std::error::Error;
use std::thread;
use std::time::Duration;
use tracing::warn;

/// Default maximum number of retry attempts for validation operations
pub const DEFAULT_MAX_RETRIES: u32 = 6;

/// Default delay between retry attempts in seconds
pub const DEFAULT_RETRY_DELAY_SECS: u64 = 4;

/// Retry a validation operation with exponential backoff.
///
/// This function will attempt the provided operation up to `max_retries` times,
/// with a gentle exponential backoff between attempts (delay grows as: base + attempt).
///
/// # Arguments
/// * `operation` - Closure that performs the validation operation
/// * `max_retries` - Maximum number of retry attempts
/// * `base_delay_secs` - Base delay in seconds (will be added to attempt number)
/// * `operation_name` - Name of the operation for logging
///
/// # Returns
/// Result from the operation if successful, or the last error if all retries fail
pub fn retry_with_backoff<F, T>(
    mut operation: F,
    max_retries: u32,
    base_delay_secs: u64,
    operation_name: &str,
) -> Result<T, Box<dyn Error>>
where
    F: FnMut() -> Result<T, Box<dyn Error>>,
{
    let mut last_error = None;

    for attempt in 1..=max_retries {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    // Gentle backoff: base + attempt (e.g., 2+1=3, 2+2=4, 2+3=5...)
                    let delay = base_delay_secs + attempt as u64;
                    warn!(
                        "{} failed (attempt {}/{}), retrying in {} seconds...",
                        operation_name, attempt, max_retries, delay
                    );
                    thread::sleep(Duration::from_secs(delay));
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "Operation failed with no error".into()))
}

/// Retry a validation operation with fixed delay.
///
/// This function will attempt the provided operation up to `max_retries` times,
/// with a fixed delay between attempts.
///
/// # Arguments
/// * `operation` - Closure that performs the validation operation
/// * `max_retries` - Maximum number of retry attempts
/// * `delay_secs` - Fixed delay in seconds between retries
/// * `operation_name` - Name of the operation for logging
///
/// # Returns
/// Result from the operation if successful, or the last error if all retries fail
pub fn retry_with_fixed_delay<F, T>(
    mut operation: F,
    max_retries: u32,
    delay_secs: u64,
    operation_name: &str,
) -> Result<T, Box<dyn Error>>
where
    F: FnMut() -> Result<T, Box<dyn Error>>,
{
    let mut last_error = None;

    for attempt in 1..=max_retries {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    warn!(
                        "{} failed (attempt {}/{}), retrying in {} seconds...",
                        operation_name, attempt, max_retries, delay_secs
                    );
                    thread::sleep(Duration::from_secs(delay_secs));
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "Operation failed with no error".into()))
}
