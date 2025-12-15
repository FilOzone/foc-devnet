# Environment Variables for FOC Localnet Containers

This document describes the environment variables that are automatically injected into the `foc-lotus`, `foc-lotus-miner`, and `foc-curio` containers when they are started.

## Network Parameters (All Containers)

These environment variables are set for **all** containers (`foc-lotus`, `foc-lotus-miner`, and `foc-curio`):

| Environment Variable | Value | Description |
|---------------------|-------|-------------|
| `FOC_LOCALNET_BLOCK_DELAY` | `2` | Block delay in seconds - controls the time between blocks in the network |
| `FOC_LOCALNET_PROPAGATION_DELAY` | `1` | Network message propagation delay in seconds |
| `FOC_LOCALNET_EQUIVOCATION_DELAY` | `0` | Time delay for equivocation checks in seconds |

### Implementation

These are defined as constants in `src/constants.rs`:
```rust
pub const FOC_LOCALNET_BLOCK_DELAY: u64 = 2;
pub const FOC_LOCALNET_PROPAGATION_DELAY: u64 = 1;
pub const FOC_LOCALNET_EQUIVOCATION_DELAY: u64 = 0;
```

And injected via the `build_network_env_vars()` helper function in `src/commands/start/env_vars.rs`.

## Contract Addresses (Curio Only)

These environment variables are set **only** for `foc-curio` containers and are populated after the FOC contracts are deployed:

| Environment Variable | Source | Description |
|---------------------|--------|-------------|
| `FOC_LOCALNET_CONTRACT_PAY` | ServiceProviderRegistry Proxy | Filecoin Pay contract address for payment processing |
| `FOC_LOCALNET_CONTRACT_FWSS` | FilecoinWarmStorageService Proxy | Filecoin Warm Storage Service contract address (whitelist service for record keepers) |
| `FOC_LOCALNET_CONTRACT_MULTICALL` | Multicall3 | Multicall3 contract address for batching multiple contract calls |
| `FOC_LOCALNET_CONTRACT_SIMPLE` | `0x0000000000000000000000000000000000000000` | Simple service address (constant zero address) |
| `FOC_LOCALNET_CONTRACT_USDFC` | MockUSDFC | USDFC token contract address for token operations |

### Implementation

These are loaded from the deployed contract addresses in `~/.foc-localnet/state/foc-contract-addresses.json` and injected via the `build_curio_contract_env_vars()` helper function in `src/commands/start/env_vars.rs`.

**Note:** If contract addresses cannot be loaded, Curio will start with a warning but without these environment variables set.

## Architecture

### Constants Module (`src/constants.rs`)

All environment variable names and constant values are defined in the constants module to avoid magic strings/numbers:

```rust
// Network parameter values
pub const FOC_LOCALNET_BLOCK_DELAY: u64 = 2;
pub const FOC_LOCALNET_PROPAGATION_DELAY: u64 = 1;
pub const FOC_LOCALNET_EQUIVOCATION_DELAY: u64 = 0;

// Contract address constant
pub const FOC_LOCALNET_CONTRACT_SIMPLE: &str = "0x0000000000000000000000000000000000000000";

// Environment variable names
pub const ENV_FOC_LOCALNET_BLOCK_DELAY: &str = "FOC_LOCALNET_BLOCK_DELAY";
pub const ENV_FOC_LOCALNET_PROPAGATION_DELAY: &str = "FOC_LOCALNET_PROPAGATION_DELAY";
pub const ENV_FOC_LOCALNET_EQUIVOCATION_DELAY: &str = "FOC_LOCALNET_EQUIVOCATION_DELAY";
pub const ENV_FOC_LOCALNET_CONTRACT_PAY: &str = "FOC_LOCALNET_CONTRACT_PAY";
pub const ENV_FOC_LOCALNET_CONTRACT_FWSS: &str = "FOC_LOCALNET_CONTRACT_FWSS";
pub const ENV_FOC_LOCALNET_CONTRACT_MULTICALL: &str = "FOC_LOCALNET_CONTRACT_MULTICALL";
pub const ENV_FOC_LOCALNET_CONTRACT_SIMPLE: &str = "FOC_LOCALNET_CONTRACT_SIMPLE";
pub const ENV_FOC_LOCALNET_CONTRACT_USDFC: &str = "FOC_LOCALNET_CONTRACT_USDFC";
```

### Environment Variables Module (`src/commands/start/env_vars.rs`)

This module provides two helper functions for building Docker environment variable arguments:

#### `build_network_env_vars() -> Vec<String>`

Returns a vector of `-e KEY=VALUE` pairs for network parameters. Used by:
- `src/commands/start/lotus/setup.rs` (Lotus daemon)
- `src/commands/start/lotus_miner/docker_command.rs` (Lotus-Miner)
- `src/commands/start/curio.rs` (Curio)

#### `build_curio_contract_env_vars() -> Result<Vec<String>, Box<dyn Error>>`

Returns a vector of `-e KEY=VALUE` pairs for contract addresses. Used only by:
- `src/commands/start/curio.rs` (Curio)

### Contract Address Mapping

The contract addresses are loaded from the `ContractAddresses` struct and mapped as follows:

| Environment Variable | Contract Name in `foc_contracts` HashMap |
|---------------------|------------------------------------------|
| `FOC_LOCALNET_CONTRACT_FWSS` | "FilecoinWarmStorageService Proxy" |
| `FOC_LOCALNET_CONTRACT_MULTICALL` | "Multicall3" |
| `FOC_LOCALNET_CONTRACT_PAY` | "ServiceProviderRegistry Proxy" |
| `FOC_LOCALNET_CONTRACT_USDFC` | From `mock_usdfc` field |
| `FOC_LOCALNET_CONTRACT_SIMPLE` | Constant: `0x0000000000000000000000000000000000000000` |

## Usage in Container Startup

### Lotus Daemon

In `src/commands/start/lotus/setup.rs`, the network environment variables are added to the Docker run command:

```rust
// Add network parameter environment variables
let network_env_vars = build_network_env_vars();
docker_args.extend(network_env_vars);
```

### Lotus-Miner

In `src/commands/start/lotus_miner/docker_command.rs`, the network environment variables are added to the Docker run command:

```rust
// Add network parameter environment variables
let network_env_vars = build_network_env_vars();
docker_args.extend(network_env_vars);
```

### Curio

In `src/commands/start/curio.rs`, both network and contract environment variables are added:

```rust
// Add network parameter environment variables (required for all nodes)
let network_env_vars = build_network_env_vars();
docker_args.extend(network_env_vars);

// Add contract address environment variables (Curio-specific)
match build_curio_contract_env_vars() {
    Ok(contract_env_vars) => {
        docker_args.extend(contract_env_vars);
    }
    Err(e) => {
        println!("    ⚠ Warning: Could not load contract addresses: {}", e);
        println!("    Curio will start without contract addresses.");
    }
}
```

## Verification

To verify that environment variables are set correctly in a running container, you can use:

```bash
# Check network parameters in any container
docker exec foc-lotus env | grep FOC_LOCALNET

# Check contract addresses in Curio
docker exec foc-curio env | grep FOC_LOCALNET_CONTRACT
```

## Future Enhancements

If additional environment variables are needed:

1. Add constant values to `src/constants.rs`
2. Add env var name constants to `src/constants.rs`
3. Update the appropriate helper function in `src/commands/start/env_vars.rs`
4. The container startup code will automatically pick up the changes
