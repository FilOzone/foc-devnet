use crossterm::style::Stylize;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tabular::{Row, Table};

use crate::port_allocator::PortAllocator;

/// Context shared across all steps during execution.
///
/// A single `StepContext` instance is created at the start of the step execution sequence
/// and can be safely shared across threads. Internal locks protect mutable state.
///
/// # Thread Safety
///
/// StepContext uses internal synchronization (Arc + Mutex) for thread-safe access:
/// - `state`: Protected by Arc<Mutex<>> for concurrent reads/writes
/// - `port_allocator`: Protected by Arc<Mutex<>> for atomic port allocation
/// - `run_id` and `logs_dir`: Immutable, safe to access from multiple threads
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
/// fn execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
///     let deployer_address = create_deployer_address()?;
///     context.set("deployer_mockusdfc_eth_address", &deployer_address);
///     Ok(())
/// }
///
/// // Step 2: USDFCDeployStep reads the address and uses it
/// fn execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
///     let deployer_address = context
///         .get("deployer_mockusdfc_eth_address")
///         .ok_or("Deployer address not found")?;
///     
///     let contract_address = deploy_contract(&deployer_address)?;
///     context.set("mock_usdfc_address", &contract_address);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct StepContext {
    /// Shared state that can be passed between steps (thread-safe)
    state: Arc<Mutex<HashMap<String, String>>>,

    /// Run ID for this execution (e.g., "251203-1246-thirsty-wolf")
    run_id: Option<String>,

    /// Run-specific logs directory (e.g., ~/.foc-localnet/logs/251203-1246-thirsty-wolf)
    logs_dir: Option<PathBuf>,

    /// Port allocator for dynamic port assignment (thread-safe)
    port_allocator: Arc<Mutex<PortAllocator>>,
}

impl Default for StepContext {
    fn default() -> Self {
        // Default to a safe port range (will be overridden by actual config)
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            run_id: None,
            logs_dir: None,
            port_allocator: Arc::new(Mutex::new(
                PortAllocator::new(5700, 300).expect("Failed to create default port allocator"),
            )),
        }
    }
}

impl StepContext {
    /// Create a new StepContext
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a StepContext with run ID, logs directory, and port allocator
    pub fn with_run_id_and_ports(
        run_id: String,
        logs_dir: PathBuf,
        port_allocator: PortAllocator,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            run_id: Some(run_id),
            logs_dir: Some(logs_dir),
            port_allocator: Arc::new(Mutex::new(port_allocator)),
        }
    }

    /// Create a StepContext with run ID and logs directory (using default port allocator)
    pub fn with_run_id(run_id: String, logs_dir: PathBuf) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            run_id: Some(run_id),
            logs_dir: Some(logs_dir),
            port_allocator: Arc::new(Mutex::new(
                PortAllocator::new(5700, 300).expect("Failed to create default port allocator"),
            )),
        }
    }

    /// Set a value in the shared state (thread-safe)
    pub fn set<K: Into<String>, V: Into<String>>(&self, key: K, value: V) {
        let mut state = self.state.lock().expect("Failed to lock state");
        state.insert(key.into(), value.into());
    }

    /// Get a value from the shared state (thread-safe)
    pub fn get(&self, key: &str) -> Option<String> {
        let state = self.state.lock().expect("Failed to lock state");
        state.get(key).cloned()
    }

    /// Get all keys matching a predicate (thread-safe)
    pub fn get_keys_matching<F>(&self, predicate: F) -> Vec<String>
    where
        F: Fn(&str) -> bool,
    {
        let state = self.state.lock().expect("Failed to lock state");
        state.keys().filter(|k| predicate(k)).cloned().collect()
    }

    /// Get the run ID
    pub fn run_id(&self) -> Option<&str> {
        self.run_id.as_deref()
    }

    /// Get the logs directory for this run
    pub fn logs_dir(&self) -> Option<&PathBuf> {
        self.logs_dir.as_ref()
    }

    /// Allocate a port from the port allocator (thread-safe)
    pub fn allocate_port(&self) -> Result<u16, Box<dyn Error>> {
        let mut allocator = self
            .port_allocator
            .lock()
            .expect("Failed to lock port allocator");
        allocator.allocate()
    }

    /// Allocate multiple ports from the port allocator (thread-safe)
    pub fn allocate_multiple_ports(&self, count: u16) -> Result<Vec<u16>, Box<dyn Error>> {
        let mut allocator = self
            .port_allocator
            .lock()
            .expect("Failed to lock port allocator");
        allocator.allocate_multiple(count)
    }

    /// Get a reference to the port allocator for verification (thread-safe)
    pub fn with_port_allocator<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&PortAllocator) -> R,
    {
        let allocator = self
            .port_allocator
            .lock()
            .expect("Failed to lock port allocator");
        f(&allocator)
    }
}

/// Trait representing a step in the startup/shutdown process
///
/// Steps must be Send + Sync to support parallel execution across threads.
pub trait Step: Send + Sync {
    /// Get the name of this step (for logging purposes)
    fn name(&self) -> &str;

    /// Pre-execution checks and setup
    ///
    /// This method should perform all necessary checks before the main execution.
    /// For example: checking if ports are available, checking if required files exist, etc.
    ///
    /// Returns Ok(()) if all checks pass, or an error describing what failed.
    fn pre_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        let _ = context; // Default implementation does nothing
        Ok(())
    }

    /// Main execution logic
    ///
    /// This method performs the actual work of the step.
    /// For example: starting a docker container, running a command, etc.
    ///
    /// Returns Ok(()) if execution was successful, or an error describing what failed.
    fn execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>>;

    /// Post-execution verification
    ///
    /// This method verifies that the execution was successful and the system is in the expected state.
    /// For example: checking if ports are accessible, verifying a service is responding, etc.
    ///
    /// Returns Ok(()) if verification passes, or an error describing what failed.
    fn post_execute(&self, context: &StepContext) -> Result<(), Box<dyn Error>> {
        let _ = context; // Default implementation does nothing
        Ok(())
    }

    /// Run the complete step (pre, execute, post)
    ///
    /// Returns the duration of the step execution.
    fn run(&self, context: &StepContext) -> Result<Duration, Box<dyn Error>> {
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
///
/// # Arguments
///
/// * `steps` - Vector of steps to execute
/// * `run_id` - Unique identifier for this run
/// * `logs_dir` - Directory for storing logs
/// * `port_start` - Starting port for the contiguous port range
/// * `port_count` - Number of ports in the range
pub fn execute_steps(
    steps: Vec<&dyn Step>,
    run_id: String,
    logs_dir: PathBuf,
    port_start: u16,
    port_count: u16,
) -> Result<(), Box<dyn Error>> {
    // Create port allocator and verify all ports are available
    let port_allocator = PortAllocator::new(port_start, port_count)?;

    println!(
        "{}",
        format!(
            "Port range check: {}-{} ({} ports)",
            port_start,
            port_start + port_count - 1,
            port_count
        )
        .cyan()
    );

    port_allocator.verify_all_ports_available()?;
    println!("{}", "  ✓ All ports in range are available".green());
    println!();

    let overall_start = Instant::now();
    let mut context = StepContext::with_run_id_and_ports(run_id, logs_dir, port_allocator);
    let mut step_timings: Vec<(String, Duration)> = Vec::new();

    for step in steps.iter() {
        println!(
            "\n{}",
            format!("=== Step: {} ===", step.name()).blue().bold()
        );
        let duration = step.run(&mut context)?;
        step_timings.push((step.name().to_string(), duration));
    }

    let overall_duration = overall_start.elapsed();

    // Step timing table
    println!("{}", "Step Execution Times:".cyan().bold());
    let mut timing_table = Table::new("{:<}  {:>}  {:>}");
    timing_table.add_row(
        Row::new()
            .with_ansi_cell("Step".bold().dark_grey())
            .with_ansi_cell("Duration".bold().dark_grey())
            .with_ansi_cell("% of Total".bold().dark_grey()),
    );

    for (step_name, duration) in &step_timings {
        let percentage = (duration.as_secs_f64() / overall_duration.as_secs_f64()) * 100.0;
        timing_table.add_row(
            Row::new()
                .with_ansi_cell(step_name.clone())
                .with_ansi_cell(format!("{:.2}s", duration.as_secs_f64()).green())
                .with_ansi_cell(format!("{:.1}%", percentage).cyan()),
        );
    }

    // Add total row
    timing_table.add_row(
        Row::new()
            .with_ansi_cell("TOTAL TIME".bold())
            .with_ansi_cell(
                format!("{:.2}s", overall_duration.as_secs_f64())
                    .green()
                    .bold(),
            )
            .with_ansi_cell("100.0%".cyan()),
    );

    print!("{}", timing_table);
    println!();

    // Print StepContext state variables
    let state = context
        .state
        .lock()
        .expect("Failed to lock state for display");
    if !state.is_empty() {
        println!("{}", "Step Context Variables:".cyan().bold());
        let mut context_table = Table::new("{:<}  {:<}");
        context_table.add_row(
            Row::new()
                .with_ansi_cell("Key".bold().dark_grey())
                .with_ansi_cell("Value".bold().dark_grey()),
        );

        // Sort keys alphabetically
        let mut keys: Vec<&String> = state.keys().collect();
        keys.sort();

        for key in keys {
            let value = state.get(key).unwrap();
            context_table.add_row(
                Row::new()
                    .with_ansi_cell(key.clone().yellow())
                    .with_ansi_cell(value.clone().dim()),
            );
        }

        print!("{}", context_table);
        println!();
    }
    drop(state); // Release lock before final output

    println!("\n{}", "All steps completed successfully!".green().bold());
    Ok(())
}

/// Execute steps organized in parallel epochs
///
/// Each epoch contains one or more steps that can run in parallel.
/// All steps in an epoch must complete successfully before moving to the next epoch.
///
/// # Arguments
///
/// * `step_epochs` - Vector of epochs, where each epoch is a vector of steps to run in parallel
/// * `run_id` - Unique identifier for this run
/// * `logs_dir` - Directory for storing logs
/// * `port_start` - Starting port for the contiguous port range
/// * `port_count` - Number of ports in the range
///
/// # Returns
///
/// Returns Ok(()) if all steps in all epochs complete successfully, or an error if any step fails.
pub fn execute_steps_parallel(
    step_epochs: Vec<Vec<&dyn Step>>,
    run_id: String,
    logs_dir: PathBuf,
    port_start: u16,
    port_count: u16,
) -> Result<(), Box<dyn Error>> {
    // Create port allocator and verify all ports are available
    let port_allocator = PortAllocator::new(port_start, port_count)?;

    println!(
        "{}",
        format!(
            "Port range check: {}-{} ({} ports)",
            port_start,
            port_start + port_count - 1,
            port_count
        )
        .cyan()
    );

    port_allocator.verify_all_ports_available()?;
    println!("{}", "  ✓ All ports in range are available".green());
    println!();

    let overall_start = Instant::now();
    let context = Arc::new(StepContext::with_run_id_and_ports(
        run_id,
        logs_dir,
        port_allocator,
    ));
    let mut all_step_timings: Vec<(String, Duration)> = Vec::new();

    for (epoch_index, epoch_steps) in step_epochs.iter().enumerate() {
        let epoch_timings = execute_epoch(epoch_index, epoch_steps, &context)?;
        all_step_timings.extend(epoch_timings);
    }

    print_execution_summary(&all_step_timings, overall_start.elapsed(), &context)
}

/// Execute a single epoch of steps (either sequentially or in parallel)
fn execute_epoch(
    epoch_index: usize,
    epoch_steps: &[&dyn Step],
    context: &Arc<StepContext>,
) -> Result<Vec<(String, Duration)>, Box<dyn Error>> {
    println!(
        "\n{}\n",
        format!(
            "EPOCH {}/{}: Running {} step(s) in parallel",
            epoch_index + 1,
            epoch_steps.len(),
            epoch_steps.len(),
        )
        .white()
        .on_dark_magenta()
        .bold()
    );

    let step_timings = if epoch_steps.len() == 1 {
        // Single step - run sequentially (no threading overhead)
        execute_single_step(epoch_steps[0], context)?
    } else {
        // Multiple steps - run in parallel
        execute_steps_in_parallel(epoch_steps, context)?
    };

    println!(
        "\n{}",
        format!("✓ Epoch {} completed successfully", epoch_index + 1)
            .green()
            .bold()
    );

    Ok(step_timings)
}

/// Execute a single step sequentially
fn execute_single_step(
    step: &dyn Step,
    context: &Arc<StepContext>,
) -> Result<Vec<(String, Duration)>, Box<dyn Error>> {
    println!(
        "\n{}",
        format!("=== Step: {} ===", step.name()).blue().bold()
    );

    let duration = step.run(context)?;
    Ok(vec![(step.name().to_string(), duration)])
}

/// Execute multiple steps in parallel using scoped threads
fn execute_steps_in_parallel(
    steps: &[&dyn Step],
    context: &Arc<StepContext>,
) -> Result<Vec<(String, Duration)>, Box<dyn Error>> {
    let mut thread_results: Vec<Result<(String, Duration), String>> = Vec::new();

    thread::scope(|s| {
        let mut handles = Vec::new();

        for &step in steps.iter() {
            let context_clone = Arc::clone(context);
            let step_name = step.name().to_string();
            let step_ref = step;

            let handle = s.spawn(move || -> (String, Result<Duration, String>) {
                let name = step_name.clone();
                let result = execute_step_with_timing(step_ref, &context_clone, &step_name);
                (name, result)
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            match handle.join() {
                Ok((step_name, result)) => {
                    thread_results.push(result.map(|d| (step_name, d)));
                }
                Err(_) => {
                    thread_results.push(Err("Thread panicked".to_string()));
                }
            }
        }
    });

    // Check results and collect timings
    let mut step_timings = Vec::new();
    for result in thread_results {
        match result {
            Ok((step_name, duration)) => {
                step_timings.push((step_name, duration));
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    Ok(step_timings)
}

/// Execute a single step with timing and error handling
fn execute_step_with_timing(
    step: &dyn Step,
    context: &StepContext,
    step_name: &str,
) -> Result<Duration, String> {
    let start_time = Instant::now();

    println!(
        "{}",
        format!("  [{}] Starting in parallel...", step_name).blue()
    );

    // Pre-execute
    println!(
        "{}",
        format!("  [{}] Running pre-execution checks...", step_name).cyan()
    );
    step.pre_execute(context)
        .map_err(|e| format!("Pre-execution failed for {}: {}", step_name, e))?;
    println!(
        "{}",
        format!("  [{}] ✓ Pre-execution checks passed", step_name).green()
    );

    // Execute
    println!("{}", format!("  [{}] Executing...", step_name).cyan());
    step.execute(context)
        .map_err(|e| format!("Execution failed for {}: {}", step_name, e))?;
    println!(
        "{}",
        format!("  [{}] ✓ Execution completed", step_name).green()
    );

    // Post-execute
    println!(
        "{}",
        format!("  [{}] Running post-execution verification...", step_name).cyan()
    );
    step.post_execute(context)
        .map_err(|e| format!("Post-execution failed for {}: {}", step_name, e))?;
    println!(
        "{}",
        format!("  [{}] ✓ Post-execution verification passed", step_name).green()
    );

    let duration = start_time.elapsed();
    println!(
        "{}",
        format!(
            "  [{}] Completed in {:.2}s",
            step_name,
            duration.as_secs_f64()
        )
        .green()
        .bold()
    );

    Ok(duration)
}

/// Print the execution summary with timing table and context variables
fn print_execution_summary(
    all_step_timings: &[(String, Duration)],
    overall_duration: Duration,
    context: &Arc<StepContext>,
) -> Result<(), Box<dyn Error>> {
    // Step timing table
    println!("{}", "Step Execution Times:".cyan().bold());
    let mut timing_table = Table::new("{:<}  {:>}  {:>}");
    timing_table.add_row(
        Row::new()
            .with_ansi_cell("Step".bold().dark_grey())
            .with_ansi_cell("Duration".bold().dark_grey())
            .with_ansi_cell("% of Total".bold().dark_grey()),
    );

    for (step_name, duration) in all_step_timings {
        let percentage = (duration.as_secs_f64() / overall_duration.as_secs_f64()) * 100.0;
        timing_table.add_row(
            Row::new()
                .with_ansi_cell(step_name.clone())
                .with_ansi_cell(format!("{:.2}s", duration.as_secs_f64()).green())
                .with_ansi_cell(format!("{:.1}%", percentage).cyan()),
        );
    }

    // Add total row
    timing_table.add_row(
        Row::new()
            .with_ansi_cell("TOTAL TIME".bold())
            .with_ansi_cell(
                format!("{:.2}s", overall_duration.as_secs_f64())
                    .green()
                    .bold(),
            )
            .with_ansi_cell("100.0%".cyan()),
    );

    print!("{}", timing_table);
    println!();

    // Print StepContext state variables
    let state = context
        .state
        .lock()
        .expect("Failed to lock state for display");
    if !state.is_empty() {
        println!("{}", "Step Context Variables:".cyan().bold());
        let mut context_table = Table::new("{:<}  {:<}");
        context_table.add_row(
            Row::new()
                .with_ansi_cell("Key".bold().dark_grey())
                .with_ansi_cell("Value".bold().dark_grey()),
        );

        // Sort keys alphabetically
        let mut keys: Vec<&String> = state.keys().collect();
        keys.sort();

        for key in keys {
            let value = state.get(key).unwrap();
            context_table.add_row(
                Row::new()
                    .with_ansi_cell(key.clone().yellow())
                    .with_ansi_cell(value.clone().dim()),
            );
        }

        print!("{}", context_table);
        println!();
    }
    drop(state); // Release lock before final output

    println!("\n{}", "All steps completed successfully!".green().bold());
    Ok(())
}
