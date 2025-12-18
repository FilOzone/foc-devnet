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
mod step;
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
pub use step::{execute_steps, execute_steps_parallel, Step, StepContext};
use usdfc_deploy::USDFCDeployStep;
use yugabyte::YugabyteStep;

use crate::commands::start::usdfc_funding::USDFCFundingStep;
use crate::config::Config;
use crate::docker::core::{container_is_running, remove_container, stop_container};
use crate::docker::{create_all_networks, start_portainer};
use crate::paths::{
    contract_addresses_file, foc_localnet_config, foc_localnet_docker_volumes,
    foc_localnet_run_logs,
};
use crate::run_id::{generate_run_id, save_current_run_id};
use crate::version_info::write_version_file;
use crossterm::style::Stylize;
pub use eth_acc_funding::constants::FEVM_ACCOUNTS_PREFUNDED;
use std::path::PathBuf;

/// Stop any existing cluster before starting a new one.
fn stop_existing_cluster() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        "Ensuring clean state by stopping any existing cluster...".yellow()
    );
    if let Err(e) = crate::commands::stop::stop_cluster() {
        println!(
            "  {} Warning: Failed to stop existing cluster: {}",
            "⚠".yellow(),
            e
        );
        println!("  Continuing with startup...");
    }
    println!();
    Ok(())
}

/// Setup directories, run ID, and version information for the cluster startup.
fn setup_directories_and_run_id(
    volumes_dir: Option<String>,
    logs_dir: Option<String>,
) -> Result<(PathBuf, PathBuf, String), Box<dyn std::error::Error>> {
    // Generate run ID for this execution
    let run_id = generate_run_id();

    // Save run ID to persistent storage
    save_current_run_id(&run_id)?;

    // Determine volumes directory
    let volumes_dir = if let Some(dir) = volumes_dir {
        PathBuf::from(dir)
    } else {
        // Create a temporary directory for volumes
        foc_localnet_docker_volumes()
    };

    // Determine logs directory - use run-specific directory
    let logs_dir = if let Some(dir) = logs_dir {
        PathBuf::from(dir)
    } else {
        foc_localnet_run_logs(&run_id)
    };

    // Create directories if they don't exist
    std::fs::create_dir_all(&volumes_dir)?;
    std::fs::create_dir_all(&logs_dir)?;

    // Write version information to the run directory
    let version_info = crate::version_info::VersionInfo::from_env();
    write_version_file(&logs_dir, &version_info)?;

    Ok((volumes_dir, logs_dir, run_id))
}

/// Perform a full regenesis reset, deleting all genesis-related files and keys.
fn perform_regenesis() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Performing regenesis (full reset)...".yellow().bold());

    // First, stop any running containers to ensure clean state
    println!("  Stopping any running containers...");
    let containers = vec!["foc-lotus-miner", "foc-lotus", "foc-curio", "foc-yugabyte"];
    for container in containers {
        if container_is_running(container)? {
            println!("    Stopping container '{}'...", container);
            stop_container(container)?;
            remove_container(container)?;
        }
    }

    let base_volumes = foc_localnet_docker_volumes();

    // Files and directories to delete
    let paths_to_delete = vec![
        base_volumes.join("lotus-keys"),
        base_volumes.join("genesis-sectors"),
        base_volumes.join("genesis").join("foc-localnet.json"),
        base_volumes.join("lotus-data"),
        base_volumes.join("lotus-miner-data"),
        crate::paths::foc_localnet_curio_volumes(),
        base_volumes.join("yugabyte-data"),
        contract_addresses_file(),
        base_volumes.join("state").join("pdp_sps"),
    ];

    for path in paths_to_delete {
        if path.exists() {
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
                println!("  {} {}", "Removed directory:".red(), path.display());
            } else {
                std::fs::remove_file(&path)?;
                println!("  {} {}", "Removed file:".red(), path.display());
            }
        } else {
            println!("  {} {}", "Skipped (not found):".dim(), path.display());
        }
    }

    println!("{}", "Regenesis complete.".green().bold());
    println!();
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
    println!("{}", "PDP Service Provider Configuration:".cyan().bold());
    println!("  • Active PDP SPs: {}", config.active_pdp_sp_count);
    println!("  • Approved PDP SPs: {}", config.approved_pdp_sp_count);
    println!();

    Ok(config)
}

/// Create all the step instances for the cluster startup sequence.
fn create_steps(volumes_dir: &PathBuf, logs_dir: &PathBuf, config: &Config) -> Vec<Box<dyn Step>> {
    let lotus_step = LotusStep::new(volumes_dir.clone(), logs_dir.clone());
    let lotus_miner_step = LotusMinerStep::new(volumes_dir.clone(), logs_dir.clone());
    let eth_acc_funding_step = ETHAccFundingStep::new(logs_dir.clone());
    let usdfc_deploy_step = USDFCDeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let usdfc_funding_step = USDFCFundingStep::new(
        volumes_dir.clone(),
        logs_dir.clone(),
        config.active_pdp_sp_count,
    );
    let multicall3_deploy_step = MultiCall3DeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let foc_deploy_step = FOCDeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let pdp_sp_reg_step = PdpSpRegistrationStep::new(
        volumes_dir.clone(),
        logs_dir.clone(),
        config.active_pdp_sp_count,
        config.approved_pdp_sp_count,
    );
    let yugabyte_step = YugabyteStep::new(
        volumes_dir.clone(),
        logs_dir.clone(),
        config.active_pdp_sp_count,
    );
    let curio_step = CurioStep::new(
        volumes_dir.clone(),
        logs_dir.clone(),
        config.active_pdp_sp_count,
    );

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
    logs_dir: &PathBuf,
    config: &Config,
) -> Vec<Vec<Box<dyn Step>>> {
    let lotus_step = LotusStep::new(volumes_dir.clone(), logs_dir.clone());
    let yugabyte_step = YugabyteStep::new(
        volumes_dir.clone(),
        logs_dir.clone(),
        config.active_pdp_sp_count,
    );
    let lotus_miner_step = LotusMinerStep::new(volumes_dir.clone(), logs_dir.clone());
    let eth_acc_funding_step = ETHAccFundingStep::new(logs_dir.clone());
    let usdfc_deploy_step = USDFCDeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let multicall3_deploy_step = MultiCall3DeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let foc_deploy_step = FOCDeployStep::new(volumes_dir.clone(), logs_dir.clone());
    let usdfc_funding_step = USDFCFundingStep::new(
        volumes_dir.clone(),
        logs_dir.clone(),
        config.active_pdp_sp_count,
    );
    let curio_step = CurioStep::new(
        volumes_dir.clone(),
        logs_dir.clone(),
        config.active_pdp_sp_count,
    );
    let pdp_sp_reg_step = PdpSpRegistrationStep::new(
        volumes_dir.clone(),
        logs_dir.clone(),
        config.active_pdp_sp_count,
        config.approved_pdp_sp_count,
    );

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
    ]
}

/// Execute the cluster startup steps.
fn execute_cluster_steps(
    volumes_dir: &PathBuf,
    logs_dir: &PathBuf,
    run_id: &str,
    config: &Config,
    parallel: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure genesis prerequisites are ready (one-time setup, needs config for sector count)
    ensure_genesis_prerequisites(config.active_pdp_sp_count)?;
    println!();

    // Validate configuration
    config
        .validate()
        .map_err(|e| format!("Configuration validation failed: {}", e))?;

    println!(
        "{}",
        format!(
            "Port allocation: {}-{} ({} ports total)",
            config.port_range_start,
            config.port_range_start + config.port_range_count - 1,
            config.port_range_count
        )
        .cyan()
    );

    println!(
        "{}",
        format!(
            "PDP SPs: {} active ({} approved)",
            config.active_pdp_sp_count, config.approved_pdp_sp_count
        )
        .cyan()
    );

    if parallel {
        println!(
            "{}",
            "Execution mode: PARALLEL (experimental)".yellow().bold()
        );
        let step_epochs = create_step_epochs(volumes_dir, logs_dir, config);

        // Convert Vec<Vec<Box<dyn Step>>> to Vec<Vec<&dyn Step>>
        let epoch_refs: Vec<Vec<&dyn Step>> = step_epochs
            .iter()
            .map(|epoch| epoch.iter().map(|s| s.as_ref()).collect())
            .collect();

        execute_steps_parallel(
            epoch_refs,
            run_id.to_string(),
            logs_dir.clone(),
            config.port_range_start,
            config.port_range_count,
        )?;
    } else {
        println!("{}", "Execution mode: SEQUENTIAL".cyan());
        let steps = create_steps(volumes_dir, logs_dir, config);

        execute_steps(
            steps.iter().map(|s| s.as_ref()).collect::<Vec<_>>(),
            run_id.to_string(),
            logs_dir.clone(),
            config.port_range_start,
            config.port_range_count,
        )?;
    }

    println!("\n{}", "Local cluster started successfully!".green().bold());
    Ok(())
}
/// Execute the start command.
///
/// This function handles starting the local Filecoin cluster.
///
/// # Arguments
///
/// * `volumes_dir` - Optional directory for docker volumes
/// * `logs_dir` - Optional directory for logs
/// * `parallel` - Whether to run steps in parallel where possible
pub fn start_cluster(
    volumes_dir: Option<String>,
    logs_dir: Option<String>,
    parallel: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    stop_existing_cluster()?;

    let (volumes_dir, logs_dir, run_id) = setup_directories_and_run_id(volumes_dir, logs_dir)?;

    // Always perform regenesis (full reset) before starting
    perform_regenesis()?;

    println!("{}", "Starting local cluster...".green().bold());
    println!("{}", format!("Run ID: {}", run_id).cyan().bold());
    println!(
        "{}",
        format!("Volumes directory: {}", volumes_dir.display()).cyan()
    );
    println!(
        "{}",
        format!("Logs directory: {}", logs_dir.display()).cyan()
    );
    println!();

    // Step 0: Create Docker networks for this run
    create_all_networks(&run_id)?;
    println!();

    // Step 0.5: Start Portainer for web UI management
    start_portainer(&run_id)?;
    println!();

    let config = load_and_validate_config()?;

    execute_cluster_steps(&volumes_dir, &logs_dir, &run_id, &config, parallel)?;

    Ok(())
}
