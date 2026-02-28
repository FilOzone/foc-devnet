use crate::commands::init::keys::load_keys;
use crate::commands::start::step::{SetupContext, Step};
use crate::constants::BUILDER_DOCKER_IMAGE;
use crate::docker::command_logger::run_and_log_command;
use crate::docker::core::docker_command;
use crate::paths::{
    devnet_info_file, foc_devnet_docker_volumes_cache, foc_devnet_synapse_sdk_repo,
};
use rand::Rng;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Seconds to wait after on-chain payment setup before running the E2E test,
/// allowing transactions to be included in a block.
const POST_SETUP_WAIT_SECONDS: u64 = 5;

/// Gas limit for cast send transactions on Filecoin FEVM.
const CAST_GAS_LIMIT: &str = "100000000";

/// 1 USDFC expressed in the token's 18-decimal base unit.
const USDFC_DEPOSIT_AMOUNT: &str = "1000000000000000000";

/// uint256 max value, used for unlimited operator approval allowances.
const MAX_UINT256: &str =
    "115792089237316195423570985008687907853269984665640564039457584007913129639935";

/// 30-day lockup period expressed in Filecoin epochs (2880 epochs/day * 30 days).
const LOCKUP_PERIOD_EPOCHS: &str = "86400";

pub struct SynapseTestE2EStep {
    #[allow(dead_code)]
    volumes_dir: PathBuf,
    run_dir: PathBuf,
    notest: bool,
}

impl SynapseTestE2EStep {
    pub fn new(volumes_dir: PathBuf, run_dir: PathBuf, notest: bool) -> Self {
        Self {
            volumes_dir,
            run_dir,
            notest,
        }
    }
}

impl Step for SynapseTestE2EStep {
    fn name(&self) -> &str {
        "Synapse E2E Test"
    }

    fn pre_execute(&self, _context: &SetupContext) -> Result<(), Box<dyn Error>> {
        if self.notest {
            info!("Skipping Synapse E2E Test (--notest flag set)");
            return Ok(());
        }

        let synapse_sdk_path = foc_devnet_synapse_sdk_repo();
        if !synapse_sdk_path.exists() {
            return Err(format!(
                "synapse-sdk repository not found at {}. Please run 'foc-devnet init' to clone it.",
                synapse_sdk_path.display()
            )
            .into());
        }
        info!(
            "synapse-sdk repository found at {}",
            synapse_sdk_path.display()
        );

        Ok(())
    }

    fn execute(&self, context: &SetupContext) -> Result<(), Box<dyn Error>> {
        if self.notest {
            return Ok(());
        }

        info!("Running Synapse E2E Test...");

        let run_id = context.run_id();
        let user_key = load_user_private_key()?;

        // Write an early devnet-info.json for the E2E test to consume. The definitive
        // version (with final startup_duration) is written by the post-steps in mod.rs.
        crate::external_api::export_devnet_info(context)?;
        let devnet_info_path = devnet_info_file(run_id);
        info!("DevNet info exported to: {}", devnet_info_path.display());

        setup_client_payments(context, &user_key)?;

        info!(
            "Waiting {} seconds for on-chain activation...",
            POST_SETUP_WAIT_SECONDS
        );
        std::thread::sleep(std::time::Duration::from_secs(POST_SETUP_WAIT_SECONDS));

        let synapse_sdk_path = foc_devnet_synapse_sdk_repo();
        let builder_volumes_dir =
            foc_devnet_docker_volumes_cache().join(crate::constants::BUILDER_CONTAINER);
        let random_file_path = create_random_test_file(&self.run_dir)?;

        let docker_args = build_docker_command(
            run_id,
            &synapse_sdk_path,
            &devnet_info_path,
            &builder_volumes_dir,
            &random_file_path,
            &user_key,
            TEST_SCRIPT,
        );

        let args_ref: Vec<&str> = docker_args.iter().map(|s| s.as_str()).collect();

        info!("Executing test in container...");
        let output = docker_command(&args_ref)?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Synapse E2E Test failed!");
            warn!("Stdout:\n{}", stdout);
            warn!("Stderr:\n{}", stderr);
            return Err("Synapse E2E Test failed".into());
        }

        info!("Synapse E2E Test completed successfully");
        Ok(())
    }

    fn post_execute(&self, _context: &SetupContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

/// Set up USER_1's wallet for FOC usage: ERC20 approve, deposit USDFC into FilecoinPay,
/// and approve FWSS as an operator with unlimited allowances and a 30-day lockup period.
/// After this, USER_1 can interact with the FOC storage services via synapse-sdk.
/// USER_2 and USER_3 are funded with USDFC but are not set up for FOC.
fn setup_client_payments(
    context: &SetupContext,
    user_key: &str,
) -> Result<(), Box<dyn Error>> {
    let lotus_rpc_url = format!(
        "http://localhost:{}/rpc/v1",
        context
            .get("lotus_api_port")
            .ok_or("lotus_api_port not found in context")?
    );
    let usdfc_addr = context
        .get("mockusdfc_contract_address")
        .ok_or("mockusdfc_contract_address not found in context")?;
    let pay_addr = context
        .get("foc_contract_filecoin_pay_v1_contract")
        .ok_or("foc_contract_filecoin_pay_v1_contract not found in context")?;
    let fwss_addr = context
        .get("foc_contract_filecoin_warm_storage_service_proxy")
        .ok_or("foc_contract_filecoin_warm_storage_service_proxy not found in context")?;
    let user_eth_addr = context
        .get("user_1_eth_address")
        .ok_or("user_1_eth_address not found in context")?;

    let run_id = context.run_id();

    info!("Approving FilecoinPay to spend USDFC...");
    cast_send(
        context,
        &format!("foc-{}-synapse-erc20-approve", run_id),
        &format!(
            "cast send {} 'approve(address,uint256)' {} {} \
             --rpc-url {} --private-key {} --gas-limit {}",
            usdfc_addr, pay_addr, USDFC_DEPOSIT_AMOUNT, lotus_rpc_url, user_key, CAST_GAS_LIMIT
        ),
        "synapse_erc20_approve",
    )?;

    info!("Depositing USDFC into FilecoinPay...");
    cast_send(
        context,
        &format!("foc-{}-synapse-fp-deposit", run_id),
        &format!(
            "cast send {} 'deposit(address,address,uint256)' {} {} {} \
             --rpc-url {} --private-key {} --gas-limit {}",
            pay_addr, usdfc_addr, user_eth_addr, USDFC_DEPOSIT_AMOUNT, lotus_rpc_url, user_key,
            CAST_GAS_LIMIT
        ),
        "synapse_fp_deposit",
    )?;

    info!("Approving FWSS as payment operator...");
    cast_send(
        context,
        &format!("foc-{}-synapse-fp-approve-operator", run_id),
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
        "synapse_fp_approve_operator",
    )?;

    info!("Client payment setup complete");
    Ok(())
}

/// Run a cast send command inside the builder container.
fn cast_send(
    context: &SetupContext,
    container_name: &str,
    cast_cmd: &str,
    log_key: &str,
) -> Result<(), Box<dyn Error>> {
    let output = run_and_log_command(
        "docker",
        &[
            "run",
            "--name",
            container_name,
            "--network",
            "host",
            BUILDER_DOCKER_IMAGE,
            "bash",
            "-c",
            cast_cmd,
        ],
        context,
        log_key,
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Cast command '{}' failed: {}", log_key, stderr).into());
    }

    Ok(())
}

/// Load the USER_1 private key from the generated keys file.
fn load_user_private_key() -> Result<String, Box<dyn Error>> {
    let keys = load_keys()?;
    let user_key = keys
        .iter()
        .find(|k| k.name == "USER_1")
        .ok_or("USER_1 key not found in addresses.json")?;

    Ok(format!("0x{}", user_key.private_key))
}

/// Build docker command arguments for test execution.
fn build_docker_command(
    run_id: &str,
    synapse_sdk_path: &Path,
    devnet_info_path: &Path,
    builder_volumes_dir: &Path,
    random_file_path: &Path,
    user_key: &str,
    script: &str,
) -> Vec<String> {
    let mut args = vec![
        "run".to_string(),
        "--name".to_string(),
        format!("foc-{}-synapse-test", run_id),
        "--network".to_string(),
        "host".to_string(),
        "-u".to_string(),
        "root".to_string(),
    ];

    // Environment variables - using NETWORK=devnet with devnet-info.json
    let env_vars = vec![
        ("NETWORK", "devnet"),
        ("DEVNET_INFO_PATH", "/devnet-info.json"),
        ("CLIENT_PRIVATE_KEY", user_key),
    ];

    for (key, value) in &env_vars {
        args.push("-e".to_string());
        args.push(format!("{}={}", key, value));
    }

    // Mount synapse-sdk
    let synapse_real = synapse_sdk_path
        .canonicalize()
        .unwrap_or_else(|_| synapse_sdk_path.to_path_buf());
    args.push("-v".to_string());
    args.push(format!("{}:/synapse-sdk", synapse_real.display()));

    // Mount devnet-info.json
    args.push("-v".to_string());
    args.push(format!(
        "{}:/devnet-info.json:ro",
        devnet_info_path.display()
    ));

    // Mount random test file
    args.push("-v".to_string());
    args.push(format!(
        "{}:/tmp/random_test_file.txt",
        random_file_path.display()
    ));

    // Mount cargo cache
    args.push("-v".to_string());
    args.push(format!(
        "{}:/root/.cargo",
        builder_volumes_dir.join("cargo").display()
    ));

    // Image and command
    args.push(BUILDER_DOCKER_IMAGE.to_string());
    args.push("/bin/bash".to_string());
    args.push("-c".to_string());
    args.push(script.to_string());

    args
}

/// Bash script that runs inside the test container: install, build, then run E2E.
const TEST_SCRIPT: &str = "\
set -e
cd /synapse-sdk
echo \"Installing dependencies...\"
pnpm install

echo \"Building SDK...\"
pnpm build

echo \"Running storage E2E test...\"
node utils/example-storage-e2e.js /tmp/random_test_file.txt";

/// Create a random test file for the E2E test.
fn create_random_test_file(run_dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let random_file_path = run_dir.join("random_test_file.txt");
    let mut file = File::create(&random_file_path)?;
    let mut rng = rand::thread_rng();
    let data: Vec<u8> = (0..912).map(|_| rng.gen()).collect();
    file.write_all(&data)?;
    info!("Created random test file at {}", random_file_path.display());
    Ok(random_file_path)
}
