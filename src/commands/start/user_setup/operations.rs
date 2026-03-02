//! Business logic for USER_1 payment setup operations.
//!
//! Sets up USER_1's wallet for FOC usage via three on-chain transactions:
//! 1. ERC20 approve – grant FilecoinPay allowance to spend USDFC
//! 2. FilecoinPay deposit – deposit USDFC into FilecoinPay on behalf of USER_1
//! 3. FilecoinPay setOperatorApproval – approve FWSS as a payment operator

use super::constants::{
    CAST_GAS_LIMIT, CONTAINER_ERC20_APPROVE, CONTAINER_FP_APPROVE_OPERATOR, CONTAINER_FP_DEPOSIT,
    LOCKUP_PERIOD_EPOCHS, MAX_UINT256, USDFC_DEPOSIT_AMOUNT,
};
use crate::commands::init::keys::load_keys;
use crate::commands::start::step::SetupContext;
use crate::constants::BUILDER_DOCKER_IMAGE;
use crate::docker::command_logger::run_and_log_command_strings;
use std::error::Error;
use tracing::info;

/// Load the USER_1 private key (hex-prefixed) from the generated keys file.
pub fn load_user_private_key() -> Result<String, Box<dyn Error>> {
    let keys = load_keys()?;
    let user_key = keys
        .iter()
        .find(|k| k.name == "USER_1")
        .ok_or("USER_1 key not found in addresses.json")?;
    Ok(format!("0x{}", user_key.private_key))
}

/// Set up USER_1's wallet for FOC usage.
///
/// Runs three sequential cast send transactions:
/// - ERC20 approve: allow FilecoinPay to spend USDFC on USER_1's behalf
/// - FilecoinPay deposit: deposit USDFC into FilecoinPay for USER_1
/// - setOperatorApproval: approve FWSS as an operator with unlimited allowances
pub fn setup_client_payments(context: &SetupContext, user_key: &str) -> Result<(), Box<dyn Error>> {
    let run_id = context.run_id();
    let lotus_rpc_url = build_lotus_rpc_url(context)?;
    let usdfc_addr = get_ctx(context, "mockusdfc_contract_address")?;
    let pay_addr = get_ctx(context, "foc_contract_filecoin_pay_v1_contract")?;
    let fwss_addr = get_ctx(context, "foc_contract_filecoin_warm_storage_service_proxy")?;
    let user_eth_addr = get_ctx(context, "user_1_eth_address")?;

    info!("Approving FilecoinPay to spend USDFC...");
    cast_send_payment(
        context,
        &format!("foc-{}-{}", run_id, CONTAINER_ERC20_APPROVE),
        &format!(
            "cast send {} 'approve(address,uint256)' {} {} \
             --rpc-url {} --private-key {} --gas-limit {}",
            usdfc_addr, pay_addr, USDFC_DEPOSIT_AMOUNT, lotus_rpc_url, user_key, CAST_GAS_LIMIT
        ),
        "user_erc20_approve",
    )?;

    info!("Depositing USDFC into FilecoinPay...");
    cast_send_payment(
        context,
        &format!("foc-{}-{}", run_id, CONTAINER_FP_DEPOSIT),
        &format!(
            "cast send {} 'deposit(address,address,uint256)' {} {} {} \
             --rpc-url {} --private-key {} --gas-limit {}",
            pay_addr,
            usdfc_addr,
            user_eth_addr,
            USDFC_DEPOSIT_AMOUNT,
            lotus_rpc_url,
            user_key,
            CAST_GAS_LIMIT
        ),
        "user_fp_deposit",
    )?;

    info!("Approving FWSS as payment operator...");
    cast_send_payment(
        context,
        &format!("foc-{}-{}", run_id, CONTAINER_FP_APPROVE_OPERATOR),
        &format!(
            "cast send {} \
            'setOperatorApproval(address,address,bool,uint256,uint256,uint256)' \
             {} {} true {} {} {} \
             --rpc-url {} --private-key {} --gas-limit {}",
            pay_addr,
            usdfc_addr,
            fwss_addr,
            MAX_UINT256,
            MAX_UINT256,
            LOCKUP_PERIOD_EPOCHS,
            lotus_rpc_url,
            user_key,
            CAST_GAS_LIMIT
        ),
        "user_fp_approve_operator",
    )?;

    info!("USER_1 client payment setup complete");
    Ok(())
}

/// Run a single cast send inside the builder container and log its output.
fn cast_send_payment(
    context: &SetupContext,
    container_name: &str,
    cast_cmd: &str,
    log_key: &str,
) -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = vec![
        "run".to_string(),
        "--name".to_string(),
        container_name.to_string(),
        "--network".to_string(),
        "host".to_string(),
        BUILDER_DOCKER_IMAGE.to_string(),
        "bash".to_string(),
        "-c".to_string(),
        cast_cmd.to_string(),
    ];

    let output = run_and_log_command_strings("docker", &args, context, log_key)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Cast command '{}' failed: {}", log_key, stderr).into());
    }
    Ok(())
}

/// Build Lotus JSON-RPC URL from the context's dynamic port.
fn build_lotus_rpc_url(context: &SetupContext) -> Result<String, Box<dyn Error>> {
    let port = get_ctx(context, "lotus_api_port")?;
    Ok(format!("http://localhost:{}/rpc/v1", port))
}

/// Retrieve a required string value from the setup context.
fn get_ctx(context: &SetupContext, key: &str) -> Result<String, Box<dyn Error>> {
    context
        .get(key)
        .ok_or_else(|| format!("{} not found in context", key).into())
}
