# Run ID Migration Checklist

## ✅ Completed

### Core Infrastructure
- [x] Run ID generation module (`src/run_id/mod.rs`)
- [x] Run ID persistence (`src/run_id/persistence.rs`)
- [x] Docker network management (`src/docker/network.rs`)
- [x] Portainer integration (`src/docker/portainer.rs`)
- [x] Container naming utilities (`src/docker/containers.rs`)
- [x] Constants updated (`src/constants.rs`)
- [x] StepContext run_id support (already existed)
- [x] `start_cluster()` command updates
- [x] `stop_cluster()` command rewrite
- [x] Project compiles successfully

## 🔄 Remaining Work

### Step Implementations (Container Name & Network Updates)

#### High Priority (Core Services)
- [ ] **Lotus Step** (`src/commands/start/lotus/`)
  - [ ] `container_management.rs` - Update container name references
  - [ ] `setup.rs` - Add `--network` flag to docker run
  - [ ] `verification.rs` - Update container name references
  - [ ] Test: Lotus starts with run-specific name and network

- [ ] **Lotus Miner Step** (`src/commands/start/lotus_miner/`)
  - [ ] Update all container name references
  - [ ] Add primary network: `porep-miner-net`
  - [ ] Connect to secondary network: `filecoin-net`
  - [ ] Test: Can communicate with Lotus

- [ ] **YugabyteDB Step** (`src/commands/start/yugabyte.rs`)
  - [ ] Update container name
  - [ ] Add network: `pdp-miner-net`
  - [ ] Test: Database accessible on expected port

- [ ] **Curio Step** (`src/commands/start/curio.rs`)
  - [ ] Update container name
  - [ ] Add primary network: `pdp-miner-net`
  - [ ] Connect to secondary network: `filecoin-net`
  - [ ] Test: Can access both YugabyteDB and Lotus

#### Medium Priority (Deployment Scripts)
- [ ] **ETH Account Funding** (`src/commands/start/eth_acc_funding/`)
  - [ ] Update builder container name
  - [ ] Change from `--network host` to `filecoin-net`
  - [ ] Update RPC URL from `localhost` to Lotus container name

- [ ] **USDFC Deploy** (`src/commands/start/usdfc_deploy/`)
  - [ ] Update builder container name
  - [ ] Change network configuration
  - [ ] Update Lotus RPC references

- [ ] **MultiCall3 Deploy** (`src/commands/start/multicall3_deploy/`)
  - [ ] Update builder container name
  - [ ] Change network configuration
  - [ ] Update Lotus RPC references

- [ ] **FOC Deploy** (`src/commands/start/foc_deploy/`)
  - [ ] Update builder container name
  - [ ] Change network configuration
  - [ ] Update Lotus RPC references

#### Lower Priority (Supporting Code)
- [ ] **FOC Deployer** (`src/commands/start/foc_deployer.rs`)
  - [ ] Review for any hardcoded container names
  - [ ] Update if necessary

- [ ] **Contract Addresses** (`src/commands/start/contract_addresses.rs`)
  - [ ] Review - likely no changes needed

### Documentation Updates
- [ ] Update `README.md` with run ID information
- [ ] Update `.github/copilot-instructions.md` with new patterns
- [ ] Update `docs/START_STOP_COMMANDS.md`
- [ ] Add network architecture diagram

### Testing
- [ ] Unit tests for new modules
- [ ] Integration test: Full start/stop cycle
- [ ] Test multiple concurrent runs
- [ ] Test Portainer web UI access
- [ ] Test container communication across networks
- [ ] Test graceful stop without run ID

## Search Commands for Finding Hardcoded Names

Run these to find remaining hardcoded references:

```bash
# Container names
rg '"foc-lotus"' src/commands/start/
rg '"foc-lotus-miner"' src/commands/start/
rg '"foc-yugabyte"' src/commands/start/
rg '"foc-curio"' src/commands/start/
rg '"foc-builder"' src/commands/start/

# Network configuration
rg '\-\-network host' src/commands/start/
rg 'localhost:1234' src/commands/start/

# Const declarations
rg 'const CONTAINER_NAME' src/commands/start/
```

## Quick Test Commands

```bash
# Build
cargo build

# Start cluster
cargo run -- start

# Check run ID saved
cat ~/.foc-localnet/state/current_runid.json

# Check networks created
docker network ls | grep $(cat ~/.foc-localnet/state/current_runid.json | jq -r .run_id | cut -d'-' -f1-4)

# Check containers running with run ID
docker ps --format "table {{.Names}}\t{{.Networks}}" | grep foc-

# Check Portainer
curl -I http://localhost:9009

# Test inter-container communication
docker exec $(docker ps -q -f name=foc-.*-lotus-miner) ping -c 1 $(docker ps --format '{{.Names}}' -f name=foc-.*-lotus)

# Stop cluster
cargo run -- stop

# Verify cleanup
docker network ls | grep foc
docker ps -a | grep foc-
cat ~/.foc-localnet/state/current_runid.json  # Should not exist
```

## File Size Policy Reminder

Per `.github/copilot-instructions.md`:
- Files should be ≤ 150 lines
- Functions should be ≤ 15 lines
- Use constants for magic numbers/names
- Extract command calls to shell module

When updating step files, split if they exceed limits.

## Next Steps

1. **Start with Lotus Step** - It's the foundation for everything else
2. **Then Lotus Miner** - Multi-network pattern is important
3. **Test those two thoroughly** - Make sure basic Filecoin network works
4. **Update YugabyteDB and Curio** - Test the PDP miner network
5. **Update builder/deployment steps** - These are similar patterns
6. **End-to-end testing** - Full cluster lifecycle
7. **Documentation updates**

## Success Criteria

- ✅ All step files updated
- ✅ No hardcoded container names (except in constants)
- ✅ All containers use user-defined networks
- ✅ Multi-run isolation works (can run `start` multiple times with different run IDs)
- ✅ Portainer shows all containers in web UI
- ✅ Containers can communicate across networks as designed
- ✅ Stop command cleans up everything properly
- ✅ Code adheres to file size policies
- ✅ All tests pass

---

**Current Status**: Core infrastructure complete, step implementations pending.

**Estimated Effort**: ~2-4 hours for step updates + testing

**Priority**: Complete Lotus and Lotus Miner steps first, then test before proceeding.
