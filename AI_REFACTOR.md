# AI Refactor Task Coordination

## Task Overview
Refactor the foc-localnet codebase according to the following policies:

### Code Quality Policies
- **File sizes**: No greater than 150 lines
- **Larger files**: Split into multi-file modules
- **Function sizes**: No greater than 15 lines
- **Magic numbers**: All magic numbers like sleep duration etc should be constants
- **Magic names**: All magic names like "foc-builder", "foc-deployer" should be constants
- **Command calls**: Refactor all `Command::new(...)` calls into a "shell" module so that nitty gritties and flags are not interspersed throughout the codebase
- **Documentation**: Each function must have a docstring describing its intent
- **Function decomposition**: Break down functions doing multiple things into smaller functions
- **Complex tasks**: Provide examples for functions undertaking complicated tasks

### Implementation Steps
1. Add policies to `.github/copilot-instructions.md`
2. Identify files exceeding 150 lines
3. Identify functions exceeding 15 lines
4. Extract constants for magic numbers and names
5. Create shell module for Command abstractions
6. Add docstrings to all functions
7. Decompose large functions
8. Update README with refactoring notes

### Progress Tracking
- [ ] Add policies to copilot-instructions.md
- [ ] Create shell module
- [ ] Extract constants
- [ ] Refactor large files (>280 lines)
- [ ] Refactor large functions (>15 lines)
- [ ] Add docstrings
- [ ] Update README

### Files to Check
- src/commands/start/mod.rs (likely large)
- src/commands/start/foc_deploy.rs
- src/docker.rs
- src/paths.rs
- Other command modules

### Constants to Extract
- Container names: foc-builder, foc-deployer, etc.
- Sleep durations
- Port numbers
- File paths
- Command flags

### Shell Module Structure
```rust
pub mod shell {
    pub fn run_command(cmd: &str, args: &[&str]) -> Result<String, Box<dyn Error>>;
    pub fn lotus_wallet_list() -> Result<Vec<String>, Box<dyn Error>>;
    // etc.
}
```