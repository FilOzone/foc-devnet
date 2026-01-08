mod curio;
mod eth_acc_funding;
mod foc_deploy;
mod foc_deployer;
mod foc_metadata;
mod genesis;
mod lotus;
mod lotus_miner;
mod lotus_utils;
mod multicall3_deploy;
mod pdp_service_provider;
pub mod step;
mod synapse_test_e2e;
mod usdfc_deploy;
mod usdfc_funding;
mod yugabyte;

use curio::CurioStep;
use eth_acc_funding::ETHAccFundingStep;
use foc_deploy::FOCDeployStep;
pub use genesis::ensure_genesis_prerequisites;
use lotus::LotusStep;
use lotus_miner::LotusMinerStep;
use multicall3_deploy::MultiCall3DeployStep;
use pdp_service_provider::PdpSpRegistrationStep;
pub use step::{execute_steps, execute_steps_parallel, SetupContext, Step};
use synapse_test_e2e::SynapseTestE2EStep;
use usdfc_deploy::USDFCDeployStep;
use yugabyte::YugabyteStep;

use crate::commands::start::usdfc_funding::USDFCFundingStep;
use crate::config::Config;
use crate::docker::core::{container_is_running, remove_container, stop_container};
use crate::docker::{create_all_networks, start_portainer};
use crate::paths::{foc_localnet_config, foc_localnet_run_dir};
use crate::run_id::save_current_run_id;
use crate::version_info::write_version_file;
pub use eth_acc_funding::constants::FEVM_ACCOUNTS_PREFUNDED;
use std::path::PathBuf;
use tracing::{info, warn};

/// Stop any existing cluster before starting a new one.
fn stop_existing_cluster() -> Result<(), Box<dyn std::error::Error>> {
    info!("Ensuring clean state by stopping any existing cluster...");
    if let Err(e) = crate::commands::stop::stop_cluster() {
        warn!("Warning: Failed to stop existing cluster: {}", e);
        info!("Continuing with startup...");
    }
    Ok(())
}

/// Setup directories, run ID, and version information for the cluster startup.
fn setup_directories_and_run_id(
    volumes_dir: Option<String>,
    _run_dir: Option<String>,
    run_id: String,
) -> Result<(PathBuf, PathBuf, String), Box<dyn std::error::Error>> {
    // Save run ID to persistent storage
    save_current_run_id(&run_id)?;

    // Determine volumes directory
    let volumes_dir = if let Some(dir) = volumes_dir {
        PathBuf::from(dir)
    } else {
        crate::paths::foc_localnet_docker_volumes_run_specific(&run_id)
    };

    // Determine run directory
    let run_dir = foc_localnet_run_dir(&run_id);

    // Create directories if they don't exist
    std::fs::create_dir_all(&volumes_dir)?;
    std::fs::create_dir_all(&run_dir)?;

    // Write version information to the run directory
    let version_info = crate::version_info::VersionInfo::from_env();
    write_version_file(&run_dir, &version_info)?;

    Ok((volumes_dir, run_dir, run_id))
}

/// Stop any running containers from previous runs.
///
/// Note: We do NOT delete old run volumes or directories since each run has
/// a unique run ID. Old runs are preserved for historical reference and debugging.
fn stop_running_containers() -> Result<(), Box<dyn std::error::Error>> {
    info!("Stopping any running containers from previous runs...");

    let containers = vec![
        crate::constants::LOTUS_MINER_CONTAINER,
        crate::constants::LOTUS_CONTAINER,
        crate::constants::CURIO_CONTAINER,
        crate::constants::YUGABYTE_CONTAINER,
    ];
    for container in containers {
        if container_is_running(container)? {
            info!("Stopping container '{}'...", container);
            stop_container(container)?;
            remove_container(container)?;
        }
    }

    info!("All running containers stopped.");
    Ok(())
}

/// Perform legacy full regenesis (deletes ALL runs - deprecated).
///
/// This function is kept for backward compatibility but should not be used
/// since it defeats the purpose of run IDs.
#[allow(dead_code)]
fn perform_regenesis_legacy() -> Result<(), Box<dyn std::error::Error>> {
    warn!("Legacy regenesis called - this deletes ALL previous runs!");

    let run_specific_volumes_root = crate::paths::foc_localnet_docker_volumes_run_specific_root();
    let runs_dir = crate::paths::foc_localnet_runs();

    // Files and directories to delete (ALL runs)
    let paths_to_delete = vec![run_specific_volumes_root, runs_dir];

    for path in paths_to_delete {
        if path.exists() {
            let result = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };

            if let Err(e) = result {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    warn!(
                        "Permission denied removing {}, trying with Docker...",
                        path.display()
                    );
                    // Fallback to Docker
                    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("/"));
                    let file_name = path.file_name().unwrap().to_string_lossy();

                    let status = std::process::Command::new("docker")
                        .args([
                            "run",
                            "-u",
                            "root",
                            "-v",
                            &format!("{}:/work", parent.display()),
                            crate::constants::BUILDER_DOCKER_IMAGE,
                            "rm",
                            "-rf",
                            &format!("/work/{}", file_name),
                        ])
                        .status()?;

                    if status.success() {
                        info!("Removed with Docker: {}", path.display());
                    } else {
                        return Err(format!(
                            "Failed to remove {} even with Docker",
                            path.display()
                        )
                        .into());
                    }
                } else {
                    return Err(e.into());
                }
            } else {
                info!("Removed: {}", path.display());
            }
        } else {
            info!("Skipped (not found): {}", path.display());
        }
    }

    info!("Regenesis complete.");
    Ok(())
}

/// Load and validate the configuration file.
fn load_and_validate_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Load config to get port range settings
    let config_path = foc_localnet_config();
    let config_content = std::fs::read_to_string(&config_path).map_err(|e| {
        format!(
            "Failed to read config file at {:?}: {}. Run 'foc-localnet init' first.",
            config_path, e
        )
    })?;
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config file at {:?}: {}", config_path, e))?;

    // Validate config
    config.validate()?;

    // Display PDP SP configuration
    info!("PDP Service Provider Configuration:");
    info!("• Active PDP SPs: {}", config.active_pdp_sp_count);
    info!("• Approved PDP SPs: {}", config.approved_pdp_sp_count);

    Ok(config)
}

/// Create all the step instances for the cluster startup sequence.
fn create_steps(
    volumes_dir: &PathBuf,
    run_dir: &PathBuf,
    config: &Config,
    notest: bool,
) -> Vec<Box<dyn Step>> {
    let lotus_step = LotusStep::new(volumes_dir.clone(), run_dir.clone());
    let lotus_miner_step = LotusMinerStep::new(volumes_dir.clone(), run_dir.clone());
    let eth_acc_funding_step = ETHAccFundingStep::new(run_dir.clone(), config.active_pdp_sp_count);
    let usdfc_deploy_step = USDFCDeployStep::new(volumes_dir.clone(), run_dir.clone());
    let usdfc_funding_step = USDFCFundingStep::new(run_dir.clone(), config.active_pdp_sp_count);
    let multicall3_deploy_step = MultiCall3DeployStep::new(volumes_dir.clone(), run_dir.clone());
    let foc_deploy_step = FOCDeployStep::new(volumes_dir.clone(), run_dir.clone());
    let pdp_sp_reg_step = PdpSpRegistrationStep::new(
        volumes_dir.clone(),
        run_dir.clone(),
        config.active_pdp_sp_count,
        config.approved_pdp_sp_count,
    );
    let yugabyte_step = YugabyteStep::new(
        volumes_dir.clone(),
        run_dir.clone(),
        config.active_pdp_sp_count,
    );
    let curio_step = CurioStep::new(
        volumes_dir.clone(),
        run_dir.clone(),
        config.active_pdp_sp_count,
    );
    let synapse_test_step = SynapseTestE2EStep::new(volumes_dir.clone(), run_dir.clone(), notest);

    // Execute all steps
    // Note: PDP SP registration MUST happen after Curio because it needs
    // the dynamic ports allocated to each Curio instance's PDP endpoint
    vec![
        Box::new(lotus_step),
        Box::new(lotus_miner_step),
        Box::new(eth_acc_funding_step),
        Box::new(usdfc_deploy_step),
        Box::new(usdfc_funding_step),
        Box::new(multicall3_deploy_step),
        Box::new(foc_deploy_step),
        Box::new(yugabyte_step),
        Box::new(curio_step),
        Box::new(pdp_sp_reg_step),
        Box::new(synapse_test_step),
    ]
}

/// Create step epochs for parallel execution.
///
/// Each epoch contains steps that can be executed in parallel.
/// All steps in an epoch must complete before the next epoch begins.
///
/// # Parallelization Strategy
///
/// - Epoch 1: Lotus + Yugabyte (independent services)
/// - Epoch 2: Lotus Miner (depends on Lotus)
/// - Epoch 3: ETH Account Funding (needs blockchain running)
/// - Epoch 4: MockUSDFC Deploy + MultiCall3 Deploy + FOC Deploy (can be parallelized)
/// - Epoch 5: MockUSDFC Funding + Curio daemons (can be parallelized, needs FOC Deploy)
/// - Epoch 6: PDP SP Registration (needs Curio daemons started)
fn create_step_epochs(
    volumes_dir: &PathBuf,
    run_dir: &PathBuf,
    config: &Config,
    notest: bool,
) -> Vec<Vec<Box<dyn Step>>> {
    let lotus_step = LotusStep::new(volumes_dir.clone(), run_dir.clone());
    let yugabyte_step = YugabyteStep::new(
        volumes_dir.clone(),
        run_dir.clone(),
        config.active_pdp_sp_count,
    );
    let lotus_miner_step = LotusMinerStep::new(volumes_dir.clone(), run_dir.clone());
    let eth_acc_funding_step = ETHAccFundingStep::new(run_dir.clone(), config.active_pdp_sp_count);
    let usdfc_deploy_step = USDFCDeployStep::new(volumes_dir.clone(), run_dir.clone());
    let multicall3_deploy_step = MultiCall3DeployStep::new(volumes_dir.clone(), run_dir.clone());
    let foc_deploy_step = FOCDeployStep::new(volumes_dir.clone(), run_dir.clone());
    let usdfc_funding_step = USDFCFundingStep::new(run_dir.clone(), config.active_pdp_sp_count);
    let curio_step = CurioStep::new(
        volumes_dir.clone(),
        run_dir.clone(),
        config.active_pdp_sp_count,
    );
    let pdp_sp_reg_step = PdpSpRegistrationStep::new(
        volumes_dir.clone(),
        run_dir.clone(),
        config.active_pdp_sp_count,
        config.approved_pdp_sp_count,
    );
    let synapse_test_step = SynapseTestE2EStep::new(volumes_dir.clone(), run_dir.clone(), notest);

    vec![
        // Epoch 1: Start Lotus
        vec![Box::new(lotus_step)],
        // Epoch 2: Start Lotus Miner (depends on Lotus)
        vec![Box::new(lotus_miner_step)],
        // Epoch 3: ETH Account Funding (needs blockchain running)
        vec![Box::new(eth_acc_funding_step)],
        // Epoch 4: Deploy contracts (can be parallelized)
        vec![
            Box::new(usdfc_deploy_step),
            Box::new(multicall3_deploy_step),
        ],
        // Epoch 5: Fund accounts with USDFC, deploy foc (needs usdfc deployed), start yugabyte for curio later
        vec![
            Box::new(foc_deploy_step),
            Box::new(usdfc_funding_step),
            Box::new(yugabyte_step),
        ],
        // Epoch 6: Start Curio daemons
        vec![Box::new(curio_step)],
        // Epoch 7: Register PDP SPs (needs Curio running, for port information)
        vec![Box::new(pdp_sp_reg_step)],
        // Epoch 8: Run Synapse E2E Test
        vec![Box::new(synapse_test_step)],
    ]
}

/// Execute the cluster startup steps.
fn execute_cluster_steps(
    volumes_dir: &PathBuf,
    run_dir: &PathBuf,
    run_id: &str,
    config: &Config,
    parallel: bool,
    portainer_port: u16,
    notest: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure genesis prerequisites are ready (one-time setup, needs config for sector count)
    ensure_genesis_prerequisites(config.active_pdp_sp_count, run_id)?;

    // Validate configuration
    config
        .validate()
        .map_err(|e| format!("Configuration validation failed: {}", e))?;

    info!(
        "Port allocation: {}-{} ({} ports total)",
        config.port_range_start,
        config.port_range_start + config.port_range_count - 1,
        config.port_range_count
    );

    info!(
        "PDP SPs: {} active ({} approved)",
        config.active_pdp_sp_count, config.approved_pdp_sp_count
    );

    if parallel {
        info!("Execution mode: PARALLEL (experimental)");
        let step_epochs = create_step_epochs(volumes_dir, run_dir, config, notest);

        // Convert Vec<Vec<Box<dyn Step>>> to Vec<Vec<&dyn Step>>
        let epoch_refs: Vec<Vec<&dyn Step>> = step_epochs
            .iter()
            .map(|epoch| epoch.iter().map(|s| s.as_ref()).collect())
            .collect();

        execute_steps_parallel(
            epoch_refs,
            run_id.to_string(),
            run_dir.clone(),
            config.port_range_start,
            config.port_range_count,
            Some(portainer_port),
            config.active_pdp_sp_count,
            config.approved_pdp_sp_count,
        )?;
    } else {
        info!("Execution mode: SEQUENTIAL");
        let steps = create_steps(volumes_dir, run_dir, config, notest);

        execute_steps(
            steps.iter().map(|s| s.as_ref()).collect::<Vec<_>>(),
            run_id.to_string(),
            run_dir.clone(),
            config.port_range_start,
            config.port_range_count,
            Some(portainer_port),
            config.active_pdp_sp_count,
            config.approved_pdp_sp_count,
        )?;
    }

    Ok(())
}

/// Start the local Filecoin network cluster.
pub fn start_cluster(
    volumes_dir: Option<String>,
    run_dir: Option<String>,
    parallel: bool,
    run_id: String,
    notest: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    stop_existing_cluster()?;

    let (volumes_dir, run_dir, run_id) =
        setup_directories_and_run_id(volumes_dir, run_dir, run_id)?;

    // Stop any running containers (but preserve old run data)
    stop_running_containers()?;

    info!("Starting local cluster...");
    info!("Run ID: {}", run_id);
    info!("Volumes directory: {}", volumes_dir.display());
    info!("Run directory: {}", run_dir.display());

    let config = load_and_validate_config()?;

    // Allocate port for Portainer (first port in dynamic range)
    let mut port_allocator = crate::port_allocator::PortAllocator::new(
        config.port_range_start,
        config.port_range_count,
    )?;
    let portainer_port = port_allocator.allocate()?;

    // Start Portainer
    start_portainer(&run_id, portainer_port)?;

    // Create networks
    create_all_networks(&run_id, config.active_pdp_sp_count)?;

    // Execute steps
    let exec_result = execute_cluster_steps(
        &volumes_dir,
        &run_dir,
        &run_id,
        &config,
        parallel,
        portainer_port,
        notest,
    );

    // Always run post-start teardown: persist logs, cleanup dead containers, write status
    if let Err(e) = finalize_start_teardown(&run_id) {
        warn!("Post-start teardown encountered an error: {}", e);
    }

    // Propagate original execution result
    exec_result?;

    info!("Cluster started successfully!");
    Ok(())
}

/// Finalize the start attempt by collecting logs, cleaning dead containers, and writing status.
fn finalize_start_teardown(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    use crate::docker::{
        persist_foc_container_logs, remove_dead_foc_containers, write_post_start_status_log,
    };

    // Persist logs for all foc* image containers
    persist_foc_container_logs(run_id)?;

    // Remove dead containers to keep environment tidy
    remove_dead_foc_containers()?;

    // Write status snapshot to the run directory
    let path = write_post_start_status_log(run_id)?;
    info!("Post-start status written to: {}", path.display());

    Ok(())
}
