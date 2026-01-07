use crate::commands::start::step::{SetupContext, Step};
use crate::constants::BUILDER_DOCKER_IMAGE;
use crate::docker::core::docker_command;
use crate::paths::{
    contract_addresses_file, foc_localnet_docker_volumes_cache, foc_localnet_keys,
    foc_localnet_synapse_sdk_repo,
};
use rand::Rng;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tracing::{info, warn};

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

        let synapse_sdk_path = foc_localnet_synapse_sdk_repo();
        if !synapse_sdk_path.exists() {
            return Err(format!(
                "synapse-sdk repository not found at {}. Please run 'foc-localnet init' to clone it.",
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
        let synapse_sdk_path = foc_localnet_synapse_sdk_repo();
        let builder_volumes_dir =
            foc_localnet_docker_volumes_cache().join(crate::constants::BUILDER_CONTAINER);

        // Load contract addresses from file
        let addresses_path = contract_addresses_file(run_id);
        let addresses_file = File::open(&addresses_path)?;
        let addresses: serde_json::Value = serde_json::from_reader(addresses_file)?;

        // Load keys from file
        let keys_path = foc_localnet_keys().join("addresses.json");
        let keys_file = File::open(&keys_path)?;
        let keys: Vec<serde_json::Value> = serde_json::from_reader(keys_file)?;

        // Get required addresses and keys
        let user_key_value = keys
            .iter()
            .find(|k| k["name"] == "USER_1")
            .ok_or("Key USER_1 not found in addresses.json")?;
        let user_key = format!(
            "0x{}",
            user_key_value["private_key"]
                .as_str()
                .ok_or("Private key is not a string")?
        );

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

        let lotus_rpc_url = crate::commands::start::lotus_utils::get_lotus_rpc_url(context)?;

        // Create random file for testing
        let random_file_path = self.run_dir.join("random_test_file.txt");
        {
            let mut file = File::create(&random_file_path)?;
            let mut rng = rand::thread_rng();
            let data: Vec<u8> = (0..912).map(|_| rng.gen()).collect();
            file.write_all(&data)?;
        }
        info!("Created random test file at {}", random_file_path.display());

        // Prepare environment variables
        let env_vars = vec![
            ("CLIENT_PRIVATE_KEY", user_key.clone()),
            ("NETWORK", "localnet".to_string()),
            ("LOCALNET_WARM_STORAGE_ADDRESS", warm_storage_addr.clone()),
            ("LOCALNET_USDFC_ADDRESS", usdfc_addr.clone()),
            ("LOCALNET_MULTICALL3_ADDRESS", multicall3_addr.clone()),
            ("LOCALNET_SP_REGISTRY_ADDRESS", sp_registry_addr.clone()),
            ("LOCALNET_RPC_URL", lotus_rpc_url.clone()),
            ("PRIVATE_KEY", user_key.clone()),
            ("CI", "true".to_string()),
        ];

        // Prepare the script to run inside the container
        let script = r#"
set -e
cd /synapse-sdk
echo "Installing dependencies..."
pnpm install

echo "Building SDK..."
pnpm build

echo "Running post-deploy setup..."
node utils/post-deploy-setup.js

echo "Waiting for 5 seconds..."
sleep 5

echo "Running storage E2E test..."
node utils/example-storage-e2e.js --network localnet /tmp/random_test_file.txt
"#;

        let mut docker_args = vec![
            "run".to_string(),
            "--name".to_string(),
            format!("foc-{}-synapse-test", run_id),
            "--network".to_string(),
            "host".to_string(),
            "-u".to_string(),
            "root".to_string(), // Run as root to ensure permissions
        ];

        // Add environment variables
        for (key, value) in env_vars {
            docker_args.push("-e".to_string());
            docker_args.push(format!("{}={}", key, value));
        }

        // Mount synapse-sdk
        // Resolve symlink to ensure Docker mounts the actual directory
        let synapse_sdk_real_path = synapse_sdk_path
            .canonicalize()
            .unwrap_or(synapse_sdk_path.clone());
        docker_args.push("-v".to_string());
        docker_args.push(format!("{}:/synapse-sdk", synapse_sdk_real_path.display()));

        // Mount random file
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

        docker_args.push(BUILDER_DOCKER_IMAGE.to_string());
        docker_args.push("/bin/bash".to_string());
        docker_args.push("-c".to_string());
        docker_args.push(script.to_string());

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

    fn post_execute(&self, _context: &SetupContext) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
