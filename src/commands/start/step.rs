use crossterm::style::Stylize;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Context shared across all steps during execution.
///
/// A single `StepContext` instance is created at the start of the step execution sequence
/// and passed as a mutable reference to each step in order. This allows steps to:
/// - Share state by writing key-value pairs that later steps can read
/// - Access common metadata like the run ID and logs directory
///
/// # Shared State Pattern
///
/// Steps should use the context to communicate important information to downstream steps:
/// - **Early steps** write values using `context.set(key, value)`
/// - **Later steps** read values using `context.get(key)`
///
/// # Example
///
/// ```rust
/// // Step 1: ETHAccFundingStep creates an address and stores it
/// fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
///     let deployer_address = create_deployer_address()?;
///     context.set("deployer_mockusdfc_eth_address", &deployer_address);
///     Ok(())
/// }
///
/// // Step 2: USDFCDeployStep reads the address and uses it
/// fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
///     let deployer_address = context
///         .get("deployer_mockusdfc_eth_address")
///         .ok_or("Deployer address not found")?;
///     
///     let contract_address = deploy_contract(deployer_address)?;
///     context.set("mock_usdfc_address", &contract_address);
///     Ok(())
/// }
///
/// // Step 3: USDFCFundingStep reads both values
/// fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
///     let deployer = context.get("deployer_mockusdfc_eth_address").unwrap();
///     let contract = context.get("mock_usdfc_address").unwrap();
///     
///     fund_users(deployer, contract)?;
///     Ok(())
/// }
/// ```
///
/// # Context Lifetime
///
/// The context lives for the entire duration of `execute_steps()` and is destroyed
/// once all steps complete. State is not persisted between different runs of the CLI.
#[derive(Debug, Default)]
pub struct StepContext {
    /// Shared state that can be passed between steps
    pub state: HashMap<String, String>,

    /// Run ID for this execution (e.g., "251203-1246-thirsty-wolf")
    pub run_id: Option<String>,

    /// Run-specific logs directory (e.g., ~/.foc-localnet/logs/251203-1246-thirsty-wolf)
    pub logs_dir: Option<PathBuf>,
}

impl StepContext {
    /// Create a new StepContext
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a StepContext with run ID and logs directory
    pub fn with_run_id(run_id: String, logs_dir: PathBuf) -> Self {
        Self {
            state: HashMap::new(),
            run_id: Some(run_id),
            logs_dir: Some(logs_dir),
        }
    }

    /// Set a value in the shared state
    pub fn set<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.state.insert(key.into(), value.into());
    }

    /// Get a value from the shared state
    pub fn get(&self, key: &str) -> Option<&String> {
        self.state.get(key)
    }

    /// Get the run ID
    pub fn run_id(&self) -> Option<&str> {
        self.run_id.as_deref()
    }

    /// Get the logs directory for this run
    pub fn logs_dir(&self) -> Option<&PathBuf> {
        self.logs_dir.as_ref()
    }
}

/// Trait representing a step in the startup/shutdown process
pub trait Step {
    /// Get the name of this step (for logging purposes)
    fn name(&self) -> &str;

    /// Pre-execution checks and setup
    ///
    /// This method should perform all necessary checks before the main execution.
    /// For example: checking if ports are available, checking if required files exist, etc.
    ///
    /// Returns Ok(()) if all checks pass, or an error describing what failed.
    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        let _ = context; // Default implementation does nothing
        Ok(())
    }

    /// Main execution logic
    ///
    /// This method performs the actual work of the step.
    /// For example: starting a docker container, running a command, etc.
    ///
    /// Returns Ok(()) if execution was successful, or an error describing what failed.
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>>;

    /// Post-execution verification
    ///
    /// This method verifies that the execution was successful and the system is in the expected state.
    /// For example: checking if ports are accessible, verifying a service is responding, etc.
    ///
    /// Returns Ok(()) if verification passes, or an error describing what failed.
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
        let _ = context; // Default implementation does nothing
        Ok(())
    }

    /// Run the complete step (pre, execute, post)
    ///
    /// Returns the duration of the step execution.
    fn run(&self, context: &mut StepContext) -> Result<Duration, Box<dyn Error>> {
        let start_time = Instant::now();

        println!(
            "{}",
            format!("Starting step: {}", self.name()).blue().bold()
        );

        println!("{}", "  Running pre-execution checks...".cyan());
        self.pre_execute(context)?;
        println!("{}", "  ✓ Pre-execution checks passed".green());

        println!("{}", "  Executing...".cyan());
        self.execute(context)?;
        println!("{}", "  ✓ Execution completed".green());

        println!("{}", "  Running post-execution verification...".cyan());
        self.post_execute(context)?;
        println!("{}", "  ✓ Post-execution verification passed".green());

        let duration = start_time.elapsed();
        println!(
            "{}",
            format!(
                "Step completed: {} ({:.2}s)",
                self.name(),
                duration.as_secs_f64()
            )
            .green()
            .bold()
        );
        Ok(duration)
    }
}

/// Execute a sequence of steps
///
/// Tracks and displays timing information for each step and overall execution.
pub fn execute_steps(
    steps: Vec<&dyn Step>,
    run_id: String,
    logs_dir: PathBuf,
) -> Result<(), Box<dyn Error>> {
    let overall_start = Instant::now();
    let mut context = StepContext::with_run_id(run_id, logs_dir);
    let mut step_timings: Vec<(String, Duration)> = Vec::new();

    for (index, step) in steps.iter().enumerate() {
        println!(
            "\n{}",
            format!(
                "=== Step {}/{}: {} ===",
                index + 1,
                steps.len(),
                step.name()
            )
            .blue()
            .bold()
        );
        let duration = step.run(&mut context)?;
        step_timings.push((step.name().to_string(), duration));
    }

    let overall_duration = overall_start.elapsed();

    // Print timing summary
    println!(
        "\n{}",
        "╔═══════════════════════════════════════════════════════════════╗"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "║                      EXECUTION SUMMARY                        ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "╠═══════════════════════════════════════════════════════════════╣"
            .cyan()
            .bold()
    );

    for (step_name, duration) in &step_timings {
        let percentage = (duration.as_secs_f64() / overall_duration.as_secs_f64()) * 100.0;
        println!(
            "{}",
            format!(
                "║ {:45} {:6.2}s ({:5.1}%) ║",
                step_name,
                duration.as_secs_f64(),
                percentage
            )
            .cyan()
        );
    }

    println!(
        "{}",
        "╠═══════════════════════════════════════════════════════════════╣"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        format!(
            "║ {:45} {:6.2}s         ║",
            "TOTAL TIME",
            overall_duration.as_secs_f64()
        )
        .green()
        .bold()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════════╝"
            .cyan()
            .bold()
    );

    println!("\n{}", "All steps completed successfully!".green().bold());
    Ok(())
}
