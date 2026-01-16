# foc-devnet - AI Coding Instructions

## Project Overview

**foc-devnet** is a Rust CLI tool for managing local Filecoin networks with FOC (Filecoin Onchain Contracts) support for warm storage services. It orchestrates Docker containers running Lotus nodes, miners, databases, and deploys smart contracts using Foundry (Forge/Cast).

**Key Technologies**: Rust, Docker, Filecoin Lotus, FEVM (Filecoin EVM), Foundry, YugabyteDB, Solidity

## Architecture Patterns

### Step-Based Execution Pattern

All startup operations follow the `Step` trait pattern (see `src/commands/start/step.rs`):

```rust
pub trait Step {
    fn name(&self) -> &str;
    fn pre_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>>;
    fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>>;
    fn post_execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>>;
    fn run(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>>;
}
```

**Why**: Provides clear separation of concerns, allows verification at each phase, and enables step-level debugging.

**Execution Flow**: Pre-checks → Main execution → Post-verification

**Context Sharing**: `StepContext` is a `HashMap<String, String>` that passes state between steps (e.g., addresses, container IDs).

### Startup Sequence

Steps execute in this strict order (see `src/commands/start/mod.rs`):

1. **Genesis Prerequisites** - Create BLS keys, pre-seal sectors, construct genesis block
2. **Lotus** - Start Filecoin daemon with FEVM enabled
3. **Lotus-Miner** - Start first-generation block producer
4. **FOC Deploy** - Deploy MockUSDFC token and FOC warm storage contracts
5. **Yugabyte** - Start database for Curio
6. **Curio** - Start second-generation miner (commented out, WIP)

**Critical**: Each step depends on previous steps being healthy. DO NOT reorder.

### Docker Container Architecture

```
foc-lotus           # Lotus daemon (ports 1234 API, 1235 P2P)
foc-lotus-miner     # First-gen miner (port 2345)
foc-builder         # Foundry tools (--network host)
foc-yugabyte        # Database for Curio (port 5433)
foc-curio           # Second-gen miner (WIP)
```

**Critical Network Details**:
- `foc-builder` uses `--network host` to access Lotus RPC at `http://localhost:1234/rpc/v1`
- All other containers use bridge networking
- Volume mounts are handled via `docker/<component>/volumes_map.toml` (TOML format)

## Directory Structure & Conventions

### User Data Directories
All persistent data lives under `~/.foc-devnet/` (see `src/paths.rs`):

```
~/.foc-devnet/
├── artifacts/
│   ├── bin/                    # Built Lotus/Curio binaries
│   └── docker/volumes/         # Container persistent data
│       ├── lotus-data/         # Lotus blockchain data
│       ├── lotus-miner-data/   # Miner data
│       ├── genesis/            # Genesis block config
│       ├── genesis-sectors/    # Pre-sealed sectors
│       ├── lotus-keys/         # BLS signing keys
│       ├── yugabyte-data/      # Database data
│       └── foc-contract-addresses.json  # Deployed contract addresses
├── logs/                       # Container logs
├── repos/
│   └── filecoin-service/       # FOC contracts repository
├── state/
│   └── .poison                 # Poison file (indicates partial startup)
└── config.toml                 # Configuration file
```

### Key Path Functions (src/paths.rs)
- `foc_devnet_home()` - Root directory `~/.foc-devnet/`
- `foc_devnet_docker_volumes()` - Docker volumes directory
- `foc_devnet_lotus_keys()` - BLS key storage
- `contract_addresses_file()` - JSON file with deployed contracts
- `foc_devnet_bin()` - Built binaries directory

## Smart Contract Deployment

### FOC Deployment Flow (src/commands/start/foc_deploy.rs)

**Fund Transfer Chain**:
```
GLOBAL_FIL_FAUCET (50,000 FIL from genesis, BLS/f3 address)
    ↓ Transfer 10,000 FIL
FEVM_FAUCET (f4/delegated address)
    ↓ Transfer 5,000 FIL
FOC_DEPLOYER (f4/delegated address that deploys contracts)
    ↓ Deploys via Foundry
MockUSDFC + FOC Service Contracts
```

**Why f4 Addresses?**: FEVM requires delegated (f4) addresses which have Ethereum compatibility. f3 (BLS) addresses cannot interact with smart contracts.

**Critical Timing**: After transferring FIL to create f4 addresses, must wait ~4 seconds for address activation on-chain.

### Contract Deployment Sequence

1. **MockUSDFC Token** (ERC-20):
   - Symbol: USDFC
   - Decimals: 18
   - Initial Supply: 1,000,000 tokens
   - Source: `contracts/MockUSDFC.sol`
   - Uses: `forge create` with `--broadcast` flag

2. **FOC Service Contracts**:
   - Deployed via `deploy-all-warm-storage.sh` script
   - Includes: PDPVerifier, ServiceProviderRegistry, FilecoinWarmStorageService, etc.
   - All addresses saved to `foc-contract-addresses.json`

### Known Issues with Contract Deployment

**Current Problem**: `forge create --broadcast` may not reliably broadcast transactions to Lotus FEVM.

**Symptoms**: 
- Transaction shows as "prepared" but no "Sending transactions" message
- No "Deployed to:" line in output
- Contract address extraction fails

**Current Workaround**: Using keystore + `cast wallet import` instead of raw `--private-key` flag

**Alternative Approaches to Consider**:
- Use `forge script` instead of `forge create`
- Use `cast send --create` for contract deployment
- Check Lotus FEVM compatibility with latest Foundry version

## Embedded Assets Pattern

Contract files, Dockerfiles, and configs are embedded at compile time (see `src/embedded_assets.rs`):

```rust
pub const MOCK_USDFC_CONTRACT: &[u8] = include_bytes!("../contracts/MockUSDFC.sol");
```

**Why**: Single binary distribution without requiring external files.

**When Adding New Assets**: 
1. Add file to appropriate directory (e.g., `contracts/`, `docker/`)
2. Add constant to `src/embedded_assets.rs`
3. Extract to temp location before use (contracts need to be on filesystem for Foundry)

## Common Development Patterns

### Error Handling
- Use `Box<dyn Error>` for flexible error propagation
- User-facing errors use `crossterm::style::Stylize`: `.red()`, `.green()`, `.yellow()`, `.cyan()`, `.bold()`
- Critical errors should be descriptive (include paths, container names, etc.)

### Lotus API Interactions
Use `Command` to execute Lotus CLI commands inside containers:

```rust
let output = Command::new("docker")
    .args(["exec", "foc-lotus", "/usr/local/bin/lotus-bins/lotus", "wallet", "list"])
    .output()?;
```

**Common Lotus Commands**:
- `lotus wallet list` - List all wallets
- `lotus wallet balance <addr>` - Check balance
- `lotus chain list` - Show chain tipsets
- `lotus-miner info` - Miner status

**JSON-RPC Alternative**: 
```bash
curl http://localhost:1234/rpc/v1 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"Filecoin.ChainHead","params":[],"id":1}'
```

### Docker Utilities (src/docker.rs)
- `container_exists(name)` - Check if container exists
- `container_is_running(name)` - Check if container is running
- `stop_and_remove_container(name)` - Clean up container
- `wait_for_port(port, timeout)` - Wait for service to be ready
- `is_port_available(port)` - Check port availability before starting

## Configuration System

### Config File Structure (config.toml)

```toml
[lotus]
location = "Git"  # or "LocalSource"
repo = "https://github.com/filecoin-project/lotus"
branch = "master"

[curio]
location = "Git"
repo = "https://github.com/filecoin-project/curio"

[filecoin-services]
location = "LocalSource"
dir = "/path/to/local/filecoin-services"
```

**Location Types** (see `src/config.rs`):
- `Git` - Clone from remote repository
- `GitTag` - Specific tag/release
- `LocalSource` - Use local directory (for development)

**When Modifying Config**:
- Always use `Config::load()` to read current config
- Use `Config::save()` after modifications
- Validate paths for `LocalSource` before saving

## Regenesis & Reset Patterns

### Regenesis (Full Reset)
Deletes: BLS keys, genesis sectors, genesis config, blockchain data, contract addresses

**When to Use**: Complete fresh start, testing genesis modifications

**Files Deleted**:
- `lotus-keys/key-1`, `lotus-keys/key-2`, `lotus-keys/prefunded-*`
- `genesis-sectors/`
- `genesis/foc-devnet.json`
- `lotus-data/`, `lotus-miner-data/`

### Reset (Chain Reset Only)
Deletes: Blockchain data, contract addresses (keeps genesis config and keys)

**When to Use**: Reset chain to block 0 while keeping same accounts

**Files Deleted**:
- `lotus-data/`, `lotus-miner-data/`
- `foc-contract-addresses.json`

## Testing & Debugging Commands

### Build & Run
```bash
cargo build                              # Dev build
cargo build --release                    # Optimized build
cargo run -- start                       # Start cluster
cargo run -- start --regenesis --reset  # Full reset + start
```

### Container Debugging
```bash
docker ps                                          # List running containers
docker logs foc-lotus                              # View Lotus logs
docker logs foc-lotus --tail 100 --follow          # Tail logs
docker exec foc-lotus /usr/local/bin/lotus-bins/lotus wallet list  # Run Lotus commands
docker inspect foc-lotus                           # Container details
```

### Manual Contract Deployment Test
```bash
# Test MockUSDFC deployment manually
docker run --rm --network host \
  -v "$(pwd)/contracts:/tmp" \
  foc-builder bash -c "
    forge create /tmp/MockUSDFC.sol:MockUSDFC \
      --rpc-url http://localhost:1234/rpc/v1 \
      --private-key <HEX_KEY> \
      --constructor-args 1000000000000000000000000 \
      --broadcast
  "
```

## Common Pitfalls & Solutions

### Port Conflicts
**Problem**: Container fails to start due to port already in use
**Solution**: Check with `lsof -i :1234` or use `foc-devnet stop` to clean up

### Volume Permission Issues
**Problem**: Container can't write to mounted volumes
**Solution**: Containers run as `foc-user:foc-user` (defined in Dockerfiles). Ensure volumes are writable.

### Genesis Timing Issues
**Problem**: f4 address not found immediately after creation
**Solution**: FEVM addresses need ~4 seconds activation time. Wait before using.

### Private Key Format Confusion
**Problem**: Lotus exports keys as hex-encoded JSON, Foundry needs raw hex
**Solution**: Use `hex::decode()` to extract actual key bytes, then re-encode to hex string

### Forge Broadcast Not Working
**Problem**: `forge create --broadcast` doesn't broadcast to Lotus FEVM
**Investigation**: Check Lotus logs for Ethereum RPC errors, verify FEVM is enabled, test with `cast send`

## Code Style & Conventions

### Code Quality Policies
- **File sizes**: No greater than 150 lines
- **Larger files**: Split into multi-file modules when exceeding 150 lines
- **Function sizes**: No greater than 15 lines
- **Magic numbers**: All magic numbers like sleep durations should be constants
- **Magic names**: All magic names like "foc-builder", "foc-deployer" should be constants
- **Command calls**: Refactor all `Command::new(...)` calls into a "shell" module so that nitty gritties and flags are not interspersed throughout the codebase
- **Documentation**: Each function must have a docstring describing its intent
- **Function decomposition**: Break down functions doing multiple things into smaller functions
- **Complex tasks**: Provide examples for functions undertaking complicated tasks

### Module Documentation
Every module should have a module-level doc comment explaining its purpose:
```rust
//! FOC deployment step.
//!
//! This module handles deploying FOC contracts to Lotus with FEVM.
```

### Error Messages
Use descriptive error messages with context:
```rust
return Err(format!(
    "Lotus container is not running. FOC deployment requires Lotus to be running with FEVM enabled. \
    Run 'foc-devnet start' to start Lotus first."
).into());
```

### Constants
Define constants at module level for magic values:
```rust
const LOTUS_RPC_PORT: u16 = 1234;
const TRANSACTION_CONFIRMATION_WAIT_SECS: u64 = 15;
const MOCK_USDFC_INITIAL_SUPPLY: &str = "1000000000000000000000000";
```

### Imports Organization
1. Standard library (`std::`)
2. External crates (`serde`, `crossterm`, etc.)
3. Internal crate modules (`crate::paths`, `crate::config`)
4. Parent/current module (`super::`, `self::`)

## Key Files Reference

- `src/commands/start/mod.rs` - Startup orchestration and step execution
- `src/commands/start/foc_deploy.rs` - FOC contract deployment logic
- `src/commands/start/step.rs` - Step trait definition
- `src/docker.rs` - Docker utility functions
- `src/paths.rs` - All path resolution functions
- `src/config.rs` - Configuration loading/saving
- `src/embedded_assets.rs` - Embedded file assets
- `docker/` - Dockerfiles and volume mappings
- `contracts/MockUSDFC.sol` - Test ERC-20 token

## Additional Resources

- **Filecoin Lotus Docs**: https://lotus.filecoin.io/
- **FEVM Documentation**: https://docs.filecoin.io/smart-contracts/fundamentals/the-fvm/
- **Foundry Book**: https://book.getfoundry.sh/
- **FOC Contracts**: https://github.com/FilOzone/filecoin-services

## When Making Changes

1. **Adding New Steps**: Implement the `Step` trait, add to startup sequence in correct order
2. **Modifying Dockerfiles**: Update embedded assets, rebuild with `foc-devnet init --rebuild`
3. **Changing Paths**: Update `src/paths.rs` and ensure backward compatibility
4. **Contract Updates**: Update embedded asset, test deployment manually first
5. **Error Handling**: Always provide context (which container, which file, which command failed)
