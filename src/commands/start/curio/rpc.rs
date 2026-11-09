//! Wait for Curio RPC readiness after storage-aware daemon initialization.

use std::error::Error;

use crate::commands::start::step::SetupContext;
use crate::docker::command_logger::run_and_log_command;
use crate::utils::retry::{retry_with_fixed_delay, DEFAULT_MAX_RETRIES, DEFAULT_RETRY_DELAY_SECS};

const CURIO_BINARY: &str = "/usr/local/bin/lotus-bins/curio";
const CURIO_RPC_PORT: u16 = 12300;

/// Wait until Curio has initialized its subsystems and serves RPC requests.
pub(super) fn wait_for_rpc(context: &SetupContext, container: &str) -> Result<(), Box<dyn Error>> {
    retry_with_fixed_delay(
        || check_rpc(context, container),
        DEFAULT_MAX_RETRIES,
        DEFAULT_RETRY_DELAY_SECS,
        &format!("Curio RPC readiness for {}", container),
    )
}

/// Probe RPC once; `wait-api` can fail immediately while the listener is starting.
fn check_rpc(context: &SetupContext, container: &str) -> Result<(), Box<dyn Error>> {
    let machine = format!("{}:{}", container, CURIO_RPC_PORT);
    let args = [
        "exec",
        container,
        CURIO_BINARY,
        "cli",
        "--machine",
        &machine,
        "wait-api",
    ];
    run(context, container, "wait_api", &args)
}

/// Execute a logged lifecycle command and retain failure context.
fn run(
    context: &SetupContext,
    container: &str,
    action: &str,
    args: &[&str],
) -> Result<(), Box<dyn Error>> {
    let key = format!("curio_{}_{}", action, container);
    let output = run_and_log_command("docker", args, context, &key)?;
    if !output.status.success() {
        return Err(format!(
            "Curio {} failed for {}: {}",
            action,
            container,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}
