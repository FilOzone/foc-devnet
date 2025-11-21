use crossterm::style::Stylize;
use std::collections::HashMap;
use std::error::Error;

/// Context shared across all steps during execution
#[derive(Debug, Default)]
pub struct StepContext {
    /// Shared state that can be passed between steps
    pub state: HashMap<String, String>,
}

impl StepContext {
    /// Create a new StepContext
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a value in the shared state
    pub fn set<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.state.insert(key.into(), value.into());
    }

    /// Get a value from the shared state
    pub fn get(&self, key: &str) -> Option<&String> {
        self.state.get(key)
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
    fn run(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
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

        println!(
            "{}",
            format!("Step completed: {}", self.name()).green().bold()
        );
        Ok(())
    }
}

/// Execute a sequence of steps
pub fn execute_steps(steps: Vec<&dyn Step>) -> Result<(), Box<dyn Error>> {
    let mut context = StepContext::new();

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
        step.run(&mut context)?;
    }

    println!("\n{}", "All steps completed successfully!".green().bold());
    Ok(())
}
