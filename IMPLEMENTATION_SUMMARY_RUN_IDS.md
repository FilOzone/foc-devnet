# Run ID Implementation Summary

## Overview
This document summarizes the implementation of run IDs for isolated cluster runs with Docker user-defined networks.

## Completed Changes

### 1. Run ID Module (`src/run_id/`)
- **mod.rs**: Core run ID generation using `chrono` and `names` crate
  - Format: `YYMMDD-HHMM-random-name` (e.g., `251203-1246-thirsty-wolf`)
- **persistence.rs**: Save/load current run ID to `~/.foc-localnet/state/current_runid.json`
  - `save_current_run_id(run_id)` - Persist run ID with timestamp
  - `load_current_run_id()` - Load current run ID or error if not found
  - `delete_current_run_id()` - Clean up after stopping cluster

### 2. Docker Network Management (`src/docker/network.rs`)
Created three user-defined bridge networks per run:
- `<RUN_ID>-filecoin-net` - For Lotus daemon containers
- `<RUN_ID>-porep-miner-net` - For Lotus miner containers (also connected to filecoin-net)
- `<RUN_ID>-pdp-miner-net` - For Curio and YugabyteDB (Curio also connects to filecoin-net)

Functions:
- `create_all_networks(run_id)` - Create all three networks
- `delete_all_networks(run_id)` - Remove all three networks
- `connect_container_to_network(container, network)` - Connect container to additional network

### 3. Portainer Management (`src/docker/portainer.rs`)
- Start Portainer on port 9009 for web UI access
- Container name: `foc-<RUN_ID>-portainer`
- Accessible at: `http://localhost:9009`
- Functions: `start_portainer(run_id)`, `stop_portainer(run_id)`

### 4. Container Naming (`src/docker/containers.rs`)
Helper functions to generate run-specific container names:
- `lotus_container_name(run_id)` → `foc-<RUN_ID>-lotus`
- `lotus_miner_container_name(run_id)` → `foc-<RUN_ID>-lotus-miner`
- `builder_container_name(run_id)` → `foc-<RUN_ID>-builder`
- `yugabyte_container_name(run_id)` → `foc-<RUN_ID>-yugabyte`
- `curio_container_name(run_id)` → `foc-<RUN_ID>-curio`
- `portainer_container_name(run_id)` → `foc-<RUN_ID>-portainer`

### 5. Updated `start_cluster()` Command
**Order of operations:**
1. Generate run ID
2. Save run ID to `~/.foc-localnet/state/current_runid.json`
3. Create directories (volumes, logs)
4. **Create Docker networks** (`create_all_networks()`)
5. **Start Portainer** (`start_portainer()`)
6. Ensure genesis prerequisites
7. Execute all startup steps (Lotus, Miner, FOC, etc.)

### 6. Updated `stop_cluster()` Command
**Reverse order of start:**
1. Load run ID from `~/.foc-localnet/state/current_runid.json`
2. Stop containers in reverse order (Curio → YugabyteDB → Miner → Lotus)
3. Stop Portainer
4. Delete Docker networks
5. Force-kill any remaining `foc-*` containers
6. Delete run ID file

**Graceful degradation**: If no run ID file exists, attempts to stop all `foc-*` containers.

### 7. StepContext Enhancement
Already supported `run_id` field:
- `StepContext::with_run_id(run_id, logs_dir)` - Create context with run ID
- `context.run_id()` - Get the current run ID
- Used by all steps to access run ID for container naming

### 8. Constants (`src/constants.rs`)
- Added `PORTAINER_PORT: u16 = 9009`
- Added `PORTAINER_CONTAINER` constant
- Container name constants remain as base names (no run ID prefix in constants)

## Remaining Work

### Steps That Need Updates
All step implementations must be updated to use run-specific container names and networks. This affects:

1. **`src/commands/start/lotus/`** (multi-file module)
   - `container_management.rs` - Change `CONTAINER_NAME` to use `lotus_container_name(run_id)`
   - `setup.rs` - Update Docker run command to use `--network` flag
   - Need to get `run_id` from `StepContext`

2. **`src/commands/start/lotus_miner/`** (multi-file module)
   - Update container name to use `lotus_miner_container_name(run_id)`
   - Connect to both `<RUN_ID>-filecoin-net` and `<RUN_ID>-porep-miner-net`

3. **`src/commands/start/yugabyte.rs`**
   - Update container name to use `yugabyte_container_name(run_id)`
   - Connect to `<RUN_ID>-pdp-miner-net`

4. **`src/commands/start/curio.rs`**
   - Update container name to use `curio_container_name(run_id)`
   - Connect to both `<RUN_ID>-pdp-miner-net` and `<RUN_ID>-filecoin-net`

5. **Builder containers in FOC deployment steps**
   - `eth_acc_funding/`, `usdfc_deploy/`, `multicall3_deploy/`, `foc_deploy/`
   - These use ephemeral `foc-builder` containers
   - Update to use `builder_container_name(run_id)`
   - Connect to `<RUN_ID>-filecoin-net` for access to Lotus RPC

## Implementation Pattern for Step Updates

### Example: Updating Lotus Step

**Before:**
```rust
const CONTAINER_NAME: &str = "foc-lotus";

pub fn start_container(docker_args: Vec<String>, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
    let output = Command::new("docker").args(&docker_args).output()?;
    // ...
}
```

**After:**
```rust
use crate::docker::containers::lotus_container_name;
use crate::docker::network::filecoin_network_name;

pub fn start_container(docker_args: Vec<String>, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
    let run_id = context.run_id().ok_or("Run ID not found in context")?;
    let container_name = lotus_container_name(run_id);
    let network_name = filecoin_network_name(run_id);
    
    // Add --network to docker_args before running
    // ...
}
```

### Docker Run Command Updates

**Old pattern:**
```bash
docker run -d --name foc-lotus ...
```

**New pattern:**
```bash
docker run -d --name foc-<RUN_ID>-lotus --network <RUN_ID>-filecoin-net ...
```

**Multi-network containers** (e.g., Lotus Miner, Curio):
```bash
docker run -d --name foc-<RUN_ID>-lotus-miner --network <RUN_ID>-porep-miner-net ...
docker network connect <RUN_ID>-filecoin-net foc-<RUN_ID>-lotus-miner
```

## Network Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Host Machine (localhost)                                     │
│                                                               │
│  Port 9009 → Portainer GUI                                  │
│  Port 1234 → Lotus RPC                                      │
│  Port 2345 → Lotus Miner API                                │
│  Port 5433 → YugabyteDB                                     │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│ <RUN_ID>-filecoin-net (Bridge Network)                      │
│                                                               │
│  ┌──────────────┐     ┌──────────────┐                      │
│  │ foc-<RID>-   │     │ foc-<RID>-   │                      │
│  │ lotus        │ ←──→│ lotus-miner  │                      │
│  │ (Daemon)     │     │              │                      │
│  └──────────────┘     └──────────────┘                      │
│         ↑                     ↑                               │
│         └─────────┬───────────┘                               │
│                   │                                           │
│            ┌──────────────┐                                  │
│            │ foc-<RID>-   │                                  │
│            │ curio        │                                  │
│            │              │                                  │
│            └──────────────┘                                  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ <RUN_ID>-porep-miner-net (Bridge Network)                   │
│                                                               │
│  ┌──────────────┐                                            │
│  │ foc-<RID>-   │                                            │
│  │ lotus-miner  │                                            │
│  │              │                                            │
│  └──────────────┘                                            │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ <RUN_ID>-pdp-miner-net (Bridge Network)                     │
│                                                               │
│  ┌──────────────┐     ┌──────────────┐                      │
│  │ foc-<RID>-   │     │ foc-<RID>-   │                      │
│  │ curio        │ ←──→│ yugabyte     │                      │
│  │              │     │              │                      │
│  └──────────────┘     └──────────────┘                      │
└─────────────────────────────────────────────────────────────┘
```

## Testing Checklist

- [ ] Build project: `cargo build`
- [ ] Test `foc-localnet start`
  - [ ] Run ID is generated and displayed
  - [ ] Run ID is saved to `~/.foc-localnet/state/current_runid.json`
  - [ ] Three networks are created
  - [ ] Portainer starts on port 9009
  - [ ] All containers have run-specific names
  - [ ] Containers can communicate across networks
- [ ] Test `foc-localnet stop`
  - [ ] Run ID is loaded correctly
  - [ ] Containers stop in reverse order
  - [ ] Networks are removed
  - [ ] Portainer is stopped
  - [ ] Run ID file is deleted
- [ ] Test multiple concurrent runs (different run IDs)
- [ ] Test stop without run ID (graceful degradation)
- [ ] Verify Portainer GUI access at http://localhost:9009

## Files Modified

1. `src/run_id/mod.rs` - Restructured as module
2. `src/run_id/persistence.rs` - NEW
3. `src/docker/mod.rs` - Added network, portainer, containers modules
4. `src/docker/network.rs` - NEW
5. `src/docker/portainer.rs` - NEW
6. `src/docker/containers.rs` - NEW
7. `src/constants.rs` - Added Portainer constants
8. `src/commands/start/mod.rs` - Network creation, Portainer, run ID persistence
9. `src/commands/stop.rs` - Complete rewrite for run ID support

## Files That Still Need Updates

1. `src/commands/start/lotus/container_management.rs`
2. `src/commands/start/lotus/setup.rs`
3. `src/commands/start/lotus_miner/*` (all files in module)
4. `src/commands/start/yugabyte.rs`
5. `src/commands/start/curio.rs`
6. `src/commands/start/eth_acc_funding/*`
7. `src/commands/start/usdfc_deploy/*`
8. `src/commands/start/multicall3_deploy/*`
9. `src/commands/start/foc_deploy/*`
10. Any other files that directly reference container names

## Notes

- **Backward Compatibility**: The stop command attempts graceful degradation when no run ID exists
- **Builder Containers**: Currently use `--network host` - need to change to user-defined networks
- **Port Exposure**: All containers should expose ports to host via `-p` flags
- **Volume Mounts**: Remain unchanged - still use `~/.foc-localnet/artifacts/docker/volumes/`
- **DNS Resolution**: Containers in same network can reach each other by container name
