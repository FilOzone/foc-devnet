# Run ID Implementation Progress

## Status: Infrastructure Complete ✅ | Lotus Step Complete ✅

## Latest Updates (Current Session)

### ✅ Completed: Lotus Step Full Migration
The Lotus step has been fully updated to use run-specific container names and networks.

**Files Modified:**
1. `src/commands/start/lotus/container_management.rs`
   - Added `get_container_name()` helper function
   - Updated `check_existing_container()` to accept `context` parameter
   - Updated `start_container()` to use run-specific container name
   - Updated `wait_for_container_init()` to use run-specific container name
   - Removed hardcoded `CONTAINER_NAME` constant

2. `src/commands/start/lotus/setup.rs`
   - Added `context` parameter to `build_docker_command()`
   - Integrated `lotus_container_name()` and `filecoin_network_name()`
   - Added `--network` flag to Docker run command
   - Removed hardcoded `CONTAINER_NAME` constant

3. `src/commands/start/lotus/verification.rs`
   - Added `get_container_name()` helper function
   - Updated `check_lotus_api()` to accept `context` parameter
   - Updated `check_ethernet_rpc()` to accept `context` parameter
   - Updated `verify_api_connectivity()` to accept `context` parameter
   - Removed hardcoded `CONTAINER_NAME` constant

4. `src/commands/start/lotus/lotus_step.rs`
   - Updated `pre_execute()` to pass context to `check_existing_container()`
   - Updated `execute()` to pass context to `build_docker_command()`
   - Updated `post_execute()` to pass context to verification functions

**Network Configuration:**
- Lotus containers now connect to `<RUN_ID>-filecoin-net`
- Container name format: `foc-<RUN_ID>-lotus`
- Ports still exposed to host: 1234 (API), 1235 (P2P)

**Build Status:** ✅ Project compiles successfully

## Remaining Work

### High Priority Steps (Core Services)

#### 1. Lotus Miner Step ⏳
**Location:** `src/commands/start/lotus_miner/`
**Complexity:** Medium-High (multi-network setup)

**Required Changes:**
- Update all files to use `lotus_miner_container_name(run_id)`
- Connect to PRIMARY network: `porep-miner-net`
- Connect to SECONDARY network: `filecoin-net` (for Lotus communication)
- Update any hardcoded references to `foc-lotus` to use `lotus_container_name(run_id)`

**Example Pattern:**
```rust
let container_name = lotus_miner_container_name(run_id);
let porep_network = porep_miner_network_name(run_id);
let filecoin_network = filecoin_network_name(run_id);

// Start with primary network
docker_args.push("--network".to_string());
docker_args.push(porep_network);

// After container starts
connect_container_to_network(&container_name, &filecoin_network)?;
```

#### 2. YugabyteDB Step ⏳
**Location:** `src/commands/start/yugabyte.rs`
**Complexity:** Low (single file, single network)

**Required Changes:**
- Replace `const CONTAINER_NAME: &str = "foc-yugabyte"` with context-based naming
- Add network: `pdp-miner-net`
- Update Docker run command to include `--network` flag

#### 3. Curio Step ⏳
**Location:** `src/commands/start/curio.rs`
**Complexity:** Medium (multi-network setup)

**Required Changes:**
- Update to use `curio_container_name(run_id)`
- Connect to PRIMARY network: `pdp-miner-net` (for YugabyteDB)
- Connect to SECONDARY network: `filecoin-net` (for Lotus FEVM access)

### Medium Priority Steps (Deployment)

#### 4. Builder Container Steps ⏳
**Locations:**
- `src/commands/start/eth_acc_funding/`
- `src/commands/start/usdfc_deploy/`
- `src/commands/start/multicall3_deploy/`
- `src/commands/start/foc_deploy/`

**Complexity:** Medium (RPC URL changes required)

**Required Changes:**
- Update to use `builder_container_name(run_id)`
- Change from `--network host` to `filecoin_network_name(run_id)`
- **CRITICAL:** Update Lotus RPC URL from `localhost:1234` to container name

**RPC URL Pattern:**
```rust
let lotus_container = lotus_container_name(run_id);
let rpc_url = format!("http://{}:1234/rpc/v1", lotus_container);
// Use rpc_url in forge/cast commands instead of http://localhost:1234/rpc/v1
```

## Quick Reference

### Network Architecture
```
<RUN_ID>-filecoin-net:
  - foc-<RUN_ID>-lotus (primary)
  - foc-<RUN_ID>-lotus-miner (secondary)
  - foc-<RUN_ID>-curio (secondary)
  - foc-<RUN_ID>-builder (ephemeral, for deployments)

<RUN_ID>-porep-miner-net:
  - foc-<RUN_ID>-lotus-miner (primary)

<RUN_ID>-pdp-miner-net:
  - foc-<RUN_ID>-curio (primary)
  - foc-<RUN_ID>-yugabyte (primary)
```

### Helper Functions Available
```rust
// Container naming
use crate::docker::containers::{
    lotus_container_name,
    lotus_miner_container_name,
    yugabyte_container_name,
    curio_container_name,
    builder_container_name,
};

// Network naming
use crate::docker::network::{
    filecoin_network_name,
    porep_miner_network_name,
    pdp_miner_network_name,
};

// Multi-network connections
use crate::docker::connect_container_to_network;
```

### Getting Run ID from Context
```rust
let run_id = context.run_id().ok_or("Run ID not found in context")?;
```

## Testing Strategy

### Unit Testing (Per Step)
After updating each step, verify compilation:
```bash
cargo build
```

### Integration Testing (After All Steps)
```bash
# Start cluster
cargo run -- start

# Verify run ID saved
cat ~/.foc-localnet/state/current_runid.json

# Verify networks created
docker network ls | grep $(jq -r .run_id ~/.foc-localnet/state/current_runid.json | cut -d'-' -f1-4)

# Verify containers using correct names
docker ps --format "table {{.Names}}\t{{.Networks}}"

# Test Portainer access
curl -I http://localhost:9009

# Stop cluster
cargo run -- stop

# Verify cleanup
docker network ls | grep foc
docker ps -a | grep foc
test -f ~/.foc-localnet/state/current_runid.json && echo "ERROR: Run ID file still exists" || echo "OK: Run ID file deleted"
```

## Next Steps

1. **Lotus Miner** - Most complex remaining step (multi-network)
2. **YugabyteDB** - Simplest remaining core step
3. **Curio** - Similar pattern to Lotus Miner
4. **Builder Steps** - All follow same pattern, can be done together
5. **Full Integration Test** - Complete start/stop cycle
6. **Documentation Updates** - Update README and guides

## Estimated Time to Completion

- **Lotus Miner:** 30-45 minutes
- **YugabyteDB:** 15 minutes
- **Curio:** 20-30 minutes
- **Builder Steps:** 45-60 minutes (all 4 steps)
- **Testing & Fixes:** 30-45 minutes

**Total Remaining:** ~2.5-3.5 hours

## Success Criteria

- ✅ All steps compile without errors
- ✅ Run ID system works end-to-end
- ✅ Networks are created and used correctly
- ✅ Containers can communicate across networks
- ✅ Portainer shows all containers
- ✅ Stop command cleans up completely
- ✅ Multiple concurrent runs possible (different run IDs)

---

**Last Updated:** 3 December 2025, 19:40 UTC
**Current Branch:** feat/integration/curio
**Compiled Successfully:** ✅ Yes
