use crate::commands::init::keys::{load_keys, KeyInfo};
use crate::commands::start::step::{SetupContext, Step};
use crate::constants::BUILDER_DOCKER_IMAGE;
use crate::docker::core::docker_command;
use crate::paths::{
    contract_addresses_file, foc_devnet_docker_volumes_cache, foc_devnet_keys,
    foc_devnet_synapse_sdk_repo,
};
use rand::Rng;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

const POST_DEPLOY_WAIT_SECONDS: u64 = 5;

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
        let synapse_sdk_path = foc_devnet_synapse_sdk_repo();
        let builder_volumes_dir =
            foc_devnet_docker_volumes_cache().join(crate::constants::BUILDER_CONTAINER);

        // Load contract addresses and keys
        let addresses = load_contract_addresses(run_id)?;
        let keys = load_wallet_keys()?;

        // Extract required addresses and keys
        let (user_key, warm_storage_addr, usdfc_addr, multicall3_addr, sp_registry_addr) =
            extract_required_addresses(&addresses, &keys)?;

        let lotus_rpc_url = crate::commands::start::lotus_utils::get_lotus_rpc_url(context)?;

        // Create random test file
        let random_file_path = create_random_test_file(&self.run_dir)?;

        // Generate the test script
        let script = generate_test_script(
            &lotus_rpc_url,
            &warm_storage_addr,
            &multicall3_addr,
            &usdfc_addr,
            &sp_registry_addr,
        );

        // Build and execute docker command
        execute_docker_test(
            run_id,
            &synapse_sdk_path,
            &builder_volumes_dir,
            &random_file_path,
            &script,
            &user_key,
            &lotus_rpc_url,
            &warm_storage_addr,
            &sp_registry_addr,
        )
    }

    fn post_execute(&self, _context: &SetupContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

/// Build and execute docker test container.
fn execute_docker_test(
    run_id: &str,
    synapse_sdk_path: &Path,
    builder_volumes_dir: &Path,
    random_file_path: &Path,
    script: &str,
    user_key: &str,
    lotus_rpc_url: &str,
    warm_storage_addr: &str,
    sp_registry_addr: &str,
) -> Result<(), Box<dyn Error>> {
    let docker_args = build_docker_command(
        run_id,
        synapse_sdk_path,
        builder_volumes_dir,
        random_file_path,
        script,
        user_key,
        lotus_rpc_url,
        warm_storage_addr,
        sp_registry_addr,
    )?;

    let args_ref: Vec<&str> = docker_args.iter().map(|s| s.as_str()).collect();

    info!("Executing test script in container...");
    let output = docker_command(&args_ref)?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Synapse E2E Test failed!");
        warn!("Stdout:\n{}", stdout);
        warn!("Stderr:\n{}", stderr);
        return Err("Synapse E2E Test failed".into());
    }

    info!("✓ Synapse E2E Test completed successfully");
    Ok(())
}

/// Build docker command arguments for test execution.
fn build_docker_command(
    run_id: &str,
    synapse_sdk_path: &Path,
    builder_volumes_dir: &Path,
    random_file_path: &Path,
    script: &str,
    user_key: &str,
    lotus_rpc_url: &str,
    warm_storage_addr: &str,
    sp_registry_addr: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut docker_args = vec![
        "run".to_string(),
        "--name".to_string(),
        format!("foc-{}-synapse-test", run_id),
        "--network".to_string(),
        "host".to_string(),
        "-u".to_string(),
        "root".to_string(),
    ];

    // Add environment variables required by synapse-sdk scripts
    let env_vars = vec![
        ("CLIENT_PRIVATE_KEY", user_key.to_string()),
        ("PRIVATE_KEY", user_key.to_string()),
        ("NETWORK", "devnet".to_string()),
        ("RPC_URL", lotus_rpc_url.to_string()),
        (
            "WARM_STORAGE_CONTRACT_ADDRESS",
            warm_storage_addr.to_string(),
        ),
        ("SP_REGISTRY_ADDRESS", sp_registry_addr.to_string()),
        ("CI", "true".to_string()),
    ];

    for (key, value) in env_vars {
        docker_args.push("-e".to_string());
        docker_args.push(format!("{}={}", key, value));
    }

    // Mount synapse-sdk
    let synapse_sdk_real_path = synapse_sdk_path
        .canonicalize()
        .unwrap_or_else(|_| synapse_sdk_path.to_path_buf());
    docker_args.push("-v".to_string());
    docker_args.push(format!("{}:/synapse-sdk", synapse_sdk_real_path.display()));

    // Mount random test file
    docker_args.push("-v".to_string());
    docker_args.push(format!(
        "{}:/tmp/random_test_file.txt",
        random_file_path.display()
    ));

    // Mount cargo cache
    docker_args.push("-v".to_string());
    docker_args.push(format!(
        "{}:/root/.cargo",
        builder_volumes_dir.join("cargo").display()
    ));

    // Add image and command
    docker_args.push(BUILDER_DOCKER_IMAGE.to_string());
    docker_args.push("/bin/bash".to_string());
    docker_args.push("-c".to_string());
    docker_args.push(script.to_string());

    Ok(docker_args)
}

/// Load contract addresses from file.
fn load_contract_addresses(run_id: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let addresses_path = contract_addresses_file(run_id);
    let addresses_file = File::open(&addresses_path)?;
    let addresses: serde_json::Value = serde_json::from_reader(addresses_file)?;
    Ok(addresses)
}

/// Load wallet keys from the generated addresses file.
fn load_wallet_keys() -> Result<Vec<KeyInfo>, Box<dyn Error>> {
    let keys_file = foc_devnet_keys().join("addresses.json");
    if !keys_file.exists() {
        return Err(format!("Keys file not found at {}", keys_file.display()).into());
    }

    load_keys()
}

/// Extract required addresses and keys from loaded data.
fn extract_required_addresses(
    addresses: &serde_json::Value,
    keys: &[KeyInfo],
) -> Result<(String, String, String, String, String), Box<dyn Error>> {
    let user_key = keys
        .iter()
        .find(|k| k.name == "USER_1")
        .ok_or("USER_1 key not found in addresses.json")?
        .private_key
        .clone();
    let user_key_prefixed = format!("0x{}", user_key);

    // Extract contract addresses
    let warm_storage_addr = addresses["foc_contracts"]["filecoin_warm_storage_service_proxy"]
        .as_str()
        .ok_or("Warm storage address not found in contract_addresses.json")?
        .to_string();
    let usdfc_addr = addresses["contracts"]["usdfc"]
        .as_str()
        .ok_or("USDFC address not found in contract_addresses.json")?
        .to_string();
    let multicall3_addr = addresses["contracts"]["multicall"]
        .as_str()
        .ok_or("Multicall3 address not found in contract_addresses.json")?
        .to_string();
    let sp_registry_addr = addresses["foc_contracts"]["service_provider_registry_proxy"]
        .as_str()
        .ok_or("SP Registry address not found in contract_addresses.json")?
        .to_string();

    Ok((
        user_key_prefixed,
        warm_storage_addr,
        usdfc_addr,
        multicall3_addr,
        sp_registry_addr,
    ))
}

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

/// Generate the shell script using CLI flags expected by synapse-sdk.
fn generate_test_script(
    lotus_rpc_url: &str,
    warm_storage_addr: &str,
    multicall3_addr: &str,
    usdfc_addr: &str,
    sp_registry_addr: &str,
) -> String {
    let mut lines = Vec::new();
    lines.extend(bootstrap_commands());
    lines.push(build_post_deploy_command(
        lotus_rpc_url,
        warm_storage_addr,
        multicall3_addr,
        usdfc_addr,
        sp_registry_addr,
    ));
    lines.extend(wait_commands());
    lines.push(build_storage_e2e_command(
        lotus_rpc_url,
        warm_storage_addr,
        multicall3_addr,
        usdfc_addr,
    ));

    lines.join("\n")
}

/// Steps to install and build the SDK inside the container.
fn bootstrap_commands() -> Vec<String> {
    vec![
        "set -e".to_string(),
        "cd /synapse-sdk".to_string(),
        "echo \"Installing dependencies...\"".to_string(),
        "pnpm install".to_string(),
        "".to_string(),
        "echo \"Building SDK...\"".to_string(),
        "pnpm build".to_string(),
        "".to_string(),
    ]
}

/// CLI invocation for post-deploy setup.
fn build_post_deploy_command(
    lotus_rpc_url: &str,
    warm_storage_addr: &str,
    multicall3_addr: &str,
    usdfc_addr: &str,
    sp_registry_addr: &str,
) -> String {
    [
        "echo \"Running post-deploy setup...\"".to_string(),
        format!(
            concat!(
                "node utils/post-deploy-setup.js \\\n",
                "    --mode client \\\n",
                "    --network devnet \\\n",
                "    --rpc-url {} \\\n",
                "    --warm-storage {} \\\n",
                "    --multicall3 {} \\\n",
                "    --usdfc {} \\\n",
                "    --sp-registry {}",
            ),
            lotus_rpc_url, warm_storage_addr, multicall3_addr, usdfc_addr, sp_registry_addr,
        ),
    ]
    .join("\n")
}

/// Simple wait between setup and test to allow on-chain activation.
fn wait_commands() -> Vec<String> {
    vec![
        format!(
            "echo \"Waiting for {} seconds...\"",
            POST_DEPLOY_WAIT_SECONDS
        ),
        format!("sleep {}", POST_DEPLOY_WAIT_SECONDS),
        "".to_string(),
    ]
}

/// CLI invocation for the storage E2E test.
fn build_storage_e2e_command(
    lotus_rpc_url: &str,
    warm_storage_addr: &str,
    multicall3_addr: &str,
    usdfc_addr: &str,
) -> String {
    format!(
        "echo \"Running storage E2E test...\"\n\
node utils/example-storage-e2e.js \\\n\
    --network devnet \\\n\
    --rpc-url {} \\\n\
    --warm-storage {} \\\n\
    --multicall3 {} \\\n\
    --usdfc {} \\\n\
    /tmp/random_test_file.txt",
        lotus_rpc_url, warm_storage_addr, multicall3_addr, usdfc_addr
    )
}
