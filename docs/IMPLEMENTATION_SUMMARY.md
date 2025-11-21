# Implementation Summary - Localnet Start/Stop

This document summarizes the implementation of the localnet startup and shutdown functionality for foc-localnet.

## Overview

The localnet now supports orchestrated startup and shutdown of all four major components:
1. **Lotus** - Execution node (FEVM/FVM)
2. **Lotus-Miner** - First-generation miner (builds tipsets, PoRep)
3. **YugabyteDB** - PostgreSQL-compatible database for Curio
4. **Curio** - Second-generation miner (PDP, no tipset building)

## Implementation Details

### New Modules Created

#### 1. `/src/commands/start/lotus.rs`
- **Purpose**: Start the Lotus execution node container
- **Container Name**: `foc-lotus`
- **Ports**: 1234 (API), 1235 (P2P)
- **Key Features**:
  - Mounts binaries, data directory, proof parameters, genesis files, and pre-sealed sectors
  - Validates all prerequisites before starting
  - Creates genesis block (`devgen.car`) on first run
  - Waits for API to be responsive before marking as complete
- **Dependencies**: Requires genesis preparation to be complete

#### 2. `/src/commands/start/lotus_miner.rs`
- **Purpose**: Start the Lotus-Miner container
- **Container Name**: `foc-lotus-miner`
- **Ports**: 2345 (API)
- **Key Features**:
  - Uses host network mode for easier communication with Lotus daemon
  - Initializes miner on first run with genesis miner actor (t01000)
  - Uses pre-sealed sectors from genesis preparation
  - Runs in `--nosync` mode for local development
- **Dependencies**: Requires Lotus daemon to be running

#### 3. `/src/commands/start/curio.rs`
- **Purpose**: Start the Curio container
- **Container Name**: `foc-curio`
- **Ports**: 12300 (API), 12301 (RPC)
- **Key Features**:
  - Uses host network mode to connect to both Lotus and YugabyteDB
  - Creates default config on first run
  - Connects to YugabyteDB at localhost:5433
- **Dependencies**: Requires both YugabyteDB and Lotus daemon to be running

### Updated Modules

#### `/src/commands/start/mod.rs`
- Added imports for all new step modules
- Updated `start_cluster()` to orchestrate all four containers in proper order:
  1. Lotus (foundational - needed by others)
  2. Lotus-Miner (builds on Lotus)
  3. YugabyteDB (database backend)
  4. Curio (needs both Lotus and YugabyteDB)

#### `/src/commands/stop.rs`
- Refactored to handle all containers
- Stops containers in reverse order (dependency-aware):
  1. Curio (depends on others)
  2. YugabyteDB (used by Curio)
  3. Lotus-Miner (uses Lotus)
  4. Lotus (foundational)
- Generalized `stop_container()` function for reusability

### Genesis Preparation (Already Implemented)

The genesis preparation module (`/src/commands/start/genesis.rs`) handles:
- Downloading Filecoin proof parameters (2048 byte sectors)
- Generating 2 BLS keys using `lotus-shed`
- Pre-sealing 2 sectors using `lotus-seed`
- All outputs cached in `~/.foc-localnet/artifacts/docker/volumes/`

### Build System

The build system in `/src/commands/build/mod.rs`:
- Already builds `lotus-shed` and `lotus-seed` alongside `lotus` and `lotus-miner`
- Uses `make 2k` for Lotus (builds 2KiB sector support)
- Explicitly runs `make lotus-shed` to build the key generation tool
- Copies all four binaries to `~/.foc-localnet/bin/`

## Architecture Notes

### Network Configuration
- **Lotus**: Uses port mappings (1234:1234, 1235:1235)
- **Lotus-Miner**: Uses host network mode for simplicity
- **YugabyteDB**: Uses port mappings for all its ports
- **Curio**: Uses host network mode to access both Lotus and YugabyteDB

### Volume Mounts
All containers mount:
- `/bin` - Binaries from `~/.foc-localnet/bin/`
- `/data` - Container-specific data directory
- Filecoin proof parameters (where needed)
- Genesis-related files (Lotus, Lotus-Miner)

### Initialization Pattern
Containers check for initialization markers (e.g., `.lotus-miner-initialized`) and only run initialization steps on first start. This allows containers to be stopped and restarted without re-initialization.

## Open Questions / Areas for Clarification

### 1. Genesis Block Construction
**Question**: The current implementation has Lotus create the genesis block during daemon startup. However, the documentation suggests using `lotus-seed genesis new` and `lotus-seed genesis set-signers` to create a `localnet.json` before starting Lotus. 

**Current Behavior**: 
- Genesis prerequisites create BLS keys and pre-sealed sectors
- Lotus daemon runs with `--lotus-make-genesis=devgen.car` and creates the genesis automatically

**Documentation Suggests**:
```bash
./lotus-seed genesis new localnet.json
./lotus-seed genesis set-signers --threshold=2 --signers <key-1> --signers <key-2> localnet.json
```

**Clarification Needed**: 
- Should we create the genesis file explicitly using `lotus-seed` in the genesis preparation step?
- If yes, how do we extract the BLS key addresses from the generated keyinfo files?
- Do we need to add a miner to the genesis using `lotus-seed genesis add-miner`?

### 2. Curio Initialization
**Question**: The Curio container currently just creates a default config and starts. Is there additional initialization needed?

**Clarification Needed**:
- Does Curio need to be connected to the Lotus daemon via configuration?
- Does it need database schema initialization in YugabyteDB?
- Are there any specific configuration parameters for local devnet mode?

### 3. Container Networking
**Question**: Currently using a mix of host networking and port mappings. 

**Current Setup**:
- Lotus: Port mappings
- Lotus-Miner: Host network
- YugabyteDB: Port mappings
- Curio: Host network

**Clarification Needed**:
- Would it be better to use a Docker bridge network for container-to-container communication?
- Are there security or isolation concerns with host networking for local development?

### 4. Lotus-Miner Wallet Import
**Question**: The documentation shows importing the genesis miner key into Lotus wallet:
```bash
./lotus wallet import --as-default ~/.genesis-sectors/pre-seal-t01000.key
```

**Clarification Needed**:
- Should this be done automatically as part of the startup sequence?
- Where is the `.key` file created during pre-sealing?
- Should it be imported into the Lotus container, the Lotus-Miner container, or both?

### 5. Container Isolation
**Question**: The requirement states "None of these containers access internet."

**Current Implementation**: No explicit network isolation configured.

**Clarification Needed**:
- Should containers run with `--network none` and then be connected via internal Docker networks?
- Is internet access required during container startup (e.g., for initialization)?
- How should proof parameters be downloaded if containers have no internet access?

### 6. Resource Limits
**Question**: Should containers have memory/CPU limits for local development?

**Clarification Needed**:
- Are there recommended resource limits for each container?
- Should these be configurable by users?

## Testing Recommendations

Before full deployment, the following should be tested:

1. **Clean Start**: Starting from `~/.foc-localnet` not existing
2. **Stop/Restart**: Verifying containers can be stopped and restarted
3. **Partial Failures**: What happens if Lotus starts but Lotus-Miner fails?
4. **Port Conflicts**: Behavior when required ports are already in use
5. **Missing Binaries**: Clear error messages when binaries aren't built
6. **Network Communication**: Verify containers can communicate with each other
7. **Genesis Validity**: Verify the genesis block is created correctly

## Next Steps

1. **Address Clarifying Questions**: Get answers to the open questions above
2. **Implement Genesis Enhancements**: If needed, update genesis preparation to use `lotus-seed genesis` commands
3. **Add Rollback Logic**: The step execution framework supports it, but explicit rollback on failure could be added
4. **Testing**: Create integration tests for the full startup/shutdown flow
5. **Documentation**: Update user-facing documentation with startup/stop commands and troubleshooting
6. **Logging**: Consider streaming container logs to files for debugging
7. **Health Checks**: Add more sophisticated health checks beyond port availability

## File Structure Summary

```
src/commands/start/
├── mod.rs              # Orchestration of all steps
├── step.rs             # Step trait and execution framework
├── genesis.rs          # Genesis preparation (one-time setup)
├── lotus.rs            # Lotus execution node step (NEW)
├── lotus_miner.rs      # Lotus-Miner step (NEW)
├── yugabyte.rs         # YugabyteDB step (existing)
└── curio.rs            # Curio step (NEW)

src/commands/
└── stop.rs             # Stop command (UPDATED for all containers)
```

## Command Usage

```bash
# Initialize the environment
foc-localnet init --lotus <path> --curio <path>

# Build binaries
foc-localnet build lotus
foc-localnet build curio

# Start the localnet (runs genesis prep automatically)
foc-localnet start

# Stop the localnet
foc-localnet stop

# Check status
foc-localnet status
```
