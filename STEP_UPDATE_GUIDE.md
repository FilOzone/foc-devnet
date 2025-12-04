# Step-by-Step Guide: Updating Container Steps to Use Run IDs

This guide provides detailed instructions for updating each step implementation to use run-specific container names and Docker networks.

## General Pattern

All step implementations need to:
1. Get the run ID from `StepContext`
2. Use container naming functions from `src/docker/containers.rs`
3. Use network naming functions from `src/docker/network.rs`
4. Add `--network` flag to `docker run` commands
5. For multi-network containers, use `docker network connect` after container start

## Step 1: Update Lotus Step

### Files to modify:
- `src/commands/start/lotus/container_management.rs`
- `src/commands/start/lotus/setup.rs`

### Changes in `container_management.rs`:

**Before:**
```rust
const CONTAINER_NAME: &str = "foc-lotus";

pub fn check_existing_container() -> Result<(), Box<dyn Error>> {
    if container_exists(CONTAINER_NAME)? {
        // ...
    }
}

pub fn start_container(docker_args: Vec<String>, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
    let output = Command::new("docker").args(&docker_args).output()?;
    // ...
}
```

**After:**
```rust
use crate::docker::containers::lotus_container_name;

pub fn get_container_name(context: &StepContext) -> Result<String, Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    Ok(lotus_container_name(run_id))
}

pub fn check_existing_container(context: &StepContext) -> Result<(), Box<dyn Error>> {
    let container_name = get_container_name(context)?;
    if container_exists(&container_name)? {
        // ... use container_name instead of CONTAINER_NAME
    }
}

pub fn start_container(docker_args: Vec<String>, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
    let output = Command::new("docker").args(&docker_args).output()?;
    // ... update all references to CONTAINER_NAME
}
```

### Changes in `setup.rs`:

**Before:**
```rust
let mut cmd_args = vec![
    "run".to_string(),
    "-d".to_string(),
    "--name".to_string(),
    "foc-lotus".to_string(),
    // ... other args
];
```

**After:**
```rust
use crate::docker::containers::lotus_container_name;
use crate::docker::network::filecoin_network_name;

let run_id = context.run_id().ok_or("Run ID not found in context")?;
let container_name = lotus_container_name(run_id);
let network_name = filecoin_network_name(run_id);

let mut cmd_args = vec![
    "run".to_string(),
    "-d".to_string(),
    "--name".to_string(),
    container_name,
    "--network".to_string(),
    network_name,
    // ... other args
];
```

## Step 2: Update Lotus Miner Step

### Files to modify:
- All files in `src/commands/start/lotus_miner/`

### Multi-network setup:
Lotus Miner needs to be in TWO networks:
1. Primary: `<RUN_ID>-porep-miner-net`
2. Secondary: `<RUN_ID>-filecoin-net` (to communicate with Lotus)

**Docker run command:**
```rust
let container_name = lotus_miner_container_name(run_id);
let porep_network = porep_miner_network_name(run_id);
let filecoin_network = filecoin_network_name(run_id);

let mut cmd_args = vec![
    "run".to_string(),
    "-d".to_string(),
    "--name".to_string(),
    container_name.clone(),
    "--network".to_string(),
    porep_network,  // Primary network
    // ... other args
];

// Start container
Command::new("docker").args(&cmd_args).output()?;

// Connect to second network
connect_container_to_network(&container_name, &filecoin_network)?;
```

## Step 3: Update YugabyteDB Step

### File: `src/commands/start/yugabyte.rs`

**Before:**
```rust
const CONTAINER_NAME: &str = "foc-yugabyte";
```

**After:**
```rust
use crate::docker::containers::yugabyte_container_name;
use crate::docker::network::pdp_miner_network_name;

// In execute function:
let run_id = context.run_id().ok_or("Run ID not found in context")?;
let container_name = yugabyte_container_name(run_id);
let network_name = pdp_miner_network_name(run_id);

let mut cmd_args = vec![
    "run".to_string(),
    "-d".to_string(),
    "--name".to_string(),
    container_name,
    "--network".to_string(),
    network_name,
    // ... rest of args
];
```

## Step 4: Update Curio Step

### File: `src/commands/start/curio.rs`

**Multi-network setup:**
Curio needs to be in TWO networks:
1. Primary: `<RUN_ID>-pdp-miner-net` (for YugabyteDB)
2. Secondary: `<RUN_ID>-filecoin-net` (for Lotus FEVM access)

**Implementation:**
```rust
use crate::docker::containers::curio_container_name;
use crate::docker::network::{pdp_miner_network_name, filecoin_network_name};
use crate::docker::connect_container_to_network;

let run_id = context.run_id().ok_or("Run ID not found in context")?;
let container_name = curio_container_name(run_id);
let pdp_network = pdp_miner_network_name(run_id);
let filecoin_network = filecoin_network_name(run_id);

let mut cmd_args = vec![
    "run".to_string(),
    "-d".to_string(),
    "--name".to_string(),
    container_name.clone(),
    "--network".to_string(),
    pdp_network,
    // ... other args
];

// Start container
Command::new("docker").args(&cmd_args).output()?;

// Connect to Filecoin network for FEVM access
connect_container_to_network(&container_name, &filecoin_network)?;
```

## Step 5: Update Builder Containers (FOC Deployment Steps)

Builder containers are used in multiple deployment steps:
- `src/commands/start/eth_acc_funding/`
- `src/commands/start/usdfc_deploy/`
- `src/commands/start/multicall3_deploy/`
- `src/commands/start/foc_deploy/`

### Current pattern (with `--network host`):
```rust
let mut cmd_args = vec![
    "run".to_string(),
    "--rm".to_string(),
    "--network".to_string(),
    "host".to_string(),
    // ...
    "foc-builder".to_string(),
];
```

### New pattern (with user-defined network):
```rust
use crate::docker::containers::builder_container_name;
use crate::docker::network::filecoin_network_name;

let run_id = context.run_id().ok_or("Run ID not found in context")?;
let container_name = builder_container_name(run_id);
let network_name = filecoin_network_name(run_id);

let mut cmd_args = vec![
    "run".to_string(),
    "--rm".to_string(),
    "--name".to_string(),
    container_name,
    "--network".to_string(),
    network_name,
    // ... NOTE: Change localhost to lotus container name for RPC access
    "foc-builder".to_string(),
];
```

### Important: Update Lotus RPC URL
When builder containers use custom networks, they can't use `http://localhost:1234/rpc/v1`.
Instead, use the Lotus container name as hostname:

**Before:**
```bash
--rpc-url http://localhost:1234/rpc/v1
```

**After:**
```rust
let lotus_container = lotus_container_name(run_id);
let rpc_url = format!("http://{}:1234/rpc/v1", lotus_container);
// Use rpc_url in forge/cast commands
```

## Quick Reference: Network Assignments

| Container         | Primary Network        | Secondary Network       | Notes                          |
|-------------------|------------------------|-------------------------|--------------------------------|
| Lotus             | filecoin-net           | -                       | RPC accessible on host:1234    |
| Lotus Miner       | porep-miner-net        | filecoin-net            | API accessible on host:2345    |
| YugabyteDB        | pdp-miner-net          | -                       | Accessible on host:5433        |
| Curio             | pdp-miner-net          | filecoin-net            | Needs both DB and FEVM         |
| Builder (ephemeral)| filecoin-net          | -                       | Accesses Lotus RPC by name     |
| Portainer         | bridge (default)       | -                       | Web UI on host:9009            |

## Testing Each Step

After updating each step, test individually:

```bash
# Start cluster
cargo run -- start

# Check networks exist
docker network ls | grep 251203  # (use your actual run ID prefix)

# Check containers are in correct networks
docker inspect foc-<RUN_ID>-lotus | grep NetworkMode
docker network inspect <RUN_ID>-filecoin-net

# Verify container can reach Lotus
docker exec foc-<RUN_ID>-lotus-miner ping -c 1 foc-<RUN_ID>-lotus

# Stop cluster
cargo run -- stop
```

## Common Pitfalls

1. **Forgetting to update function signatures**: If a function needs `context`, add it to parameters
2. **Container name references in logs**: Search for hardcoded container names in log messages
3. **Port bindings**: Still use `-p 1234:1234` to expose to host
4. **Volume mounts**: Don't change these - they still use absolute paths from host
5. **Builder RPC URL**: Must change from `localhost` to container name when using custom networks

## Search for Hardcoded Names

Use these commands to find remaining hardcoded container names:

```bash
grep -r '"foc-lotus"' src/commands/start/
grep -r '"foc-lotus-miner"' src/commands/start/
grep -r '"foc-yugabyte"' src/commands/start/
grep -r '"foc-curio"' src/commands/start/
grep -r '"foc-builder"' src/commands/start/
grep -r 'localhost:1234' src/commands/start/
```

## Checklist Before Committing

- [ ] All hardcoded container names replaced with `*_container_name(run_id)` calls
- [ ] All `docker run` commands have `--network` flag
- [ ] Multi-network containers use `connect_container_to_network()`
- [ ] Builder containers use Lotus container name for RPC instead of `localhost`
- [ ] All functions that need context have it in parameters
- [ ] Code compiles: `cargo build`
- [ ] Basic test: `cargo run -- start && cargo run -- stop`
- [ ] Verify networks created: `docker network ls`
- [ ] Verify Portainer accessible: `http://localhost:9009`
