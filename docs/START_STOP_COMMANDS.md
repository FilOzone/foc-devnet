# Start and Stop Commands Implementation

## Overview

The `start` and `stop` commands have been implemented with a robust, step-based architecture that supports multi-phase execution with comprehensive validation.

## Architecture

### Step Framework

A new `Step` trait has been introduced in `src/commands/start/step.rs` that defines a three-phase execution model:

1. **Pre-Execution**: Validates preconditions before execution
2. **Execution**: Performs the actual operation
3. **Post-Execution**: Verifies the operation completed successfully

Each step implements the `Step` trait:

```rust
pub trait Step {
    fn name(&self) -> &str;
    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>>;
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>>;
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>>;
}
```

The framework includes:
- **StepContext**: Shared state container for passing data between steps
- **execute_steps()**: Orchestrates execution of multiple steps in sequence
- Comprehensive error handling at each phase
- Visual feedback with colored output

## YugabyteDB Step

### Pre-Execution Checks

1. **Container Status**: Checks if `foc-yugabyte` container already exists
   - If running: stops and removes it
   - If stopped: removes it
   - Ensures clean state before starting

2. **Port Availability**: Verifies all required ports are free:
   - 5433 - YSQL (PostgreSQL API)
   - 9042 - YCQL (Cassandra API)
   - 7000 - YB-Master RPC
   - 9000 - YB-Master Admin UI
   - 7100 - YB-TServer RPC
   - 9100 - YB-TServer Admin UI
   - 15433 - YugabyteDB Web UI

3. **Docker Image**: Confirms `foc-foc-yugabyte` image exists

### Execution

1. Creates volume directory for persistent data storage
2. Runs Docker container with:
   - Name: `foc-yugabyte`
   - All required port mappings
   - Volume mount for data persistence
   - Detached mode (-d)

### Post-Execution Verification

1. **Container Health**: Verifies container is running
2. **Port Accessibility**: Tests each port accepts connections (30s timeout per port)
3. **PostgreSQL Connectivity**: Executes test query to verify database is ready
4. **User Information**: Displays access URLs for Web UI and PostgreSQL

## Stop Command

The `stop` command performs reverse operations:

1. **Check Container Existence**: Determines if container exists
2. **Stop Container**: Gracefully stops running container
3. **Verify Stop**: Confirms container is no longer running
4. **Remove Container**: Removes stopped container
5. **Verify Removal**: Confirms container is completely removed

### Edge Cases Handled

- Container doesn't exist: Reports informational message, continues
- Container exists but not running: Skips stop, proceeds to removal
- Errors during stop/removal: Provides detailed error messages

## Usage

### Start the Cluster

```bash
foc-localnet start
```

Optional parameters:
```bash
foc-localnet start --volumes-dir /path/to/volumes --logs-dir /path/to/logs
```

### Stop the Cluster

```bash
foc-localnet stop
```

## Output Examples

### Successful Start

```
Starting local cluster...
Volumes directory: /tmp/foc-localnet-volumes
Logs directory: /home/user/.foc-localnet/logs

=== Step 1/1: Start YugabyteDB ===
Starting step: Start YugabyteDB
  Running pre-execution checks...
    ✓ All required ports are available
    ✓ Docker image 'foc-foc-yugabyte' found
  ✓ Pre-execution checks passed
  Executing...
    Starting container 'foc-yugabyte'...
    ✓ Container started with ID: eec156091d42
  ✓ Execution completed
  Running post-execution verification...
    Waiting for YugabyteDB to start...
    ✓ Container is running
    Verifying port accessibility...
      Checking port 5433 (YSQL (PostgreSQL API))... ✓
      Checking port 9042 (YCQL (Cassandra API))... ✓
      Checking port 7000 (YB-Master RPC)... ✓
      Checking port 9000 (YB-Master Admin UI)... ✓
      Checking port 7100 (YB-TServer RPC)... ✓
      Checking port 9100 (YB-TServer Admin UI)... ✓
      Checking port 15433 (YugabyteDB Web UI)... ✓
    Verifying PostgreSQL connectivity...
    ✓ PostgreSQL is ready and accepting queries

    ✓ YugabyteDB is ready!
      Web UI: http://localhost:15433
      PostgreSQL: localhost:5433
  ✓ Post-execution verification passed
Step completed: Start YugabyteDB

All steps completed successfully!

Local cluster started successfully!
```

### Successful Stop

```
Stopping local cluster...

Stopping YugabyteDB...
  Stopping container 'foc-yugabyte'...
  ✓ Container stopped
  Removing container 'foc-yugabyte'...
  ✓ Container removed
YugabyteDB stopped successfully

Local cluster stopped successfully!
```

## Future Extensions

The step-based architecture supports easy addition of new components:

1. Create new step implementing the `Step` trait
2. Add to the steps vector in `start_cluster()`
3. Steps execute sequentially with full validation

Example future steps:
- Lotus node startup
- Lotus miner startup
- Curio node startup
- Network initialization
- Genesis block creation

Each step follows the same three-phase pattern with comprehensive validation and error handling.

## Files Modified/Created

- `src/commands/start/mod.rs` - Main start command orchestration
- `src/commands/start/step.rs` - Step trait and execution framework
- `src/commands/start/yugabyte.rs` - YugabyteDB step implementation
- `src/commands/stop.rs` - Enhanced stop command
- Removed: `src/commands/start.rs` (converted to module)
