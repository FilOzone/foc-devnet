# PDP Service Provider Registration Step

## Overview

This implementation adds a new step to the foc-localnet startup sequence that:
1. Registers the `PDP_SP_0` account as a service provider in the `ServiceProviderRegistry` contract
2. Adds the provider to the approved providers list in the `FilecoinWarmStorageService` contract
3. Stores the provider ID to `~/.foc-localnet/state/pdp_sp_0.provider_id.json`

## Architecture

The implementation follows the Step pattern used throughout foc-localnet and adheres to the code quality policies:

### Module Structure

```
src/commands/start/pdp_service_provider/
├── mod.rs                          # Module exports
├── pdp_service_provider_step.rs    # Step implementation (140 lines)
├── constants.rs                    # Configuration constants (32 lines)
├── provider_id.rs                  # State management (47 lines)
└── registration.rs                 # Contract interactions (196 lines)
```

All files are under 150 lines as required.

### Key Components

#### 1. Constants (`constants.rs`)
- Registration fee: 5 FIL
- Transaction confirmation wait: 8 seconds
- PDP service configuration:
  - Service URL: `http://localhost:8080`
  - Min piece size: 1 KiB
  - Max piece size: 1 GiB
  - Storage price: 1 FIL per TiB per day
  - Min proving period: 2880 epochs (~1 day)
  - Location: "LocalNet"

#### 2. Provider ID State (`provider_id.rs`)
- `ProviderIdInfo` struct for serialization
- Methods: `load()`, `save()`, `exists()`
- Stored at: `~/.foc-localnet/state/pdp_sp_0.provider_id.json`

#### 3. Contract Interactions (`registration.rs`)
- `register_provider()` - Calls `ServiceProviderRegistry.registerProvider()`
  - Sends 5 FIL registration fee
  - Provides PDP capability keys/values
  - Queries and returns provider ID
- `add_to_approved_list()` - Calls `FilecoinWarmStorageService.addApprovedProvider()`
  - Must be called by contract owner (DEPLOYER_FOC)
  - Adds provider to approved list for automatic selection

#### 4. Step Implementation (`pdp_service_provider_step.rs`)
Implements the `Step` trait:
- `pre_execute()` - Validates Lotus is running, checks addresses, verifies contracts
- `execute()` - Registers provider, adds to approved list, saves state
- `post_execute()` - Verifies provider ID file was created successfully

## Startup Sequence Position

The step is inserted after FOC contract deployment and before Yugabyte:

1. Lotus
2. Lotus-Miner
3. ETH Account Funding
4. USDFC Deploy
5. USDFC Funding
6. MultiCall3 Deploy
7. FOC Deploy
8. **PDP Service Provider Registration** ← NEW
9. Yugabyte
10. Curio

## Contract Interactions

### ServiceProviderRegistry.registerProvider()

```solidity
function registerProvider(
    address payee,              // PDP_SP_0 ETH address (same as provider)
    string calldata name,       // "PDP_SP_0"
    string calldata description,// "PDP Service Provider 0 for LocalNet"
    ProductType productType,    // 0 (PDP)
    string[] calldata capabilityKeys,
    bytes[] calldata capabilityValues
) external payable returns (uint256 providerId)
```

**Capability Keys/Values:**
- `serviceURL`: `"http://localhost:8080"` (bytes-encoded)
- `minPieceSizeInBytes`: `1024` (uint64, big-endian encoded)
- `maxPieceSizeInBytes`: `1073741824` (uint64, big-endian encoded)
- `storagePricePerTibPerDay`: `1000000000000000000` (uint64, big-endian encoded)
- `minProvingPeriodInEpochs`: `2880` (uint64, big-endian encoded)
- `location`: `"LocalNet"` (bytes-encoded)
- `paymentTokenAddress`: MockUSDFC contract address (hex-encoded)

### FilecoinWarmStorageService.addApprovedProvider()

```solidity
function addApprovedProvider(uint256 providerId) external onlyOwner
```

Called by DEPLOYER_FOC to add the provider to the approved list.

## Context Variables

### Input (from ETHAccFundingStep)
- `pdp_sp_0_address` - Filecoin f4 address
- `pdp_sp_0_eth_address` - Ethereum 0x address
- `deployer_foc_address` - FOC deployer f4 address
- `deployer_foc_eth_address` - FOC deployer 0x address

### Output (for downstream steps)
- `pdp_sp_0_provider_id` - Registered provider ID (uint64 as string)

## State Files

### Provider ID State
**Path:** `~/.foc-localnet/state/pdp_sp_0.provider_id.json`

**Format:**
```json
{
  "provider_id": 1,
  "provider_address": "0x...",
  "payee_address": "0x..."
}
```

This file can be used by:
- Curio for provider configuration
- User scripts for deal-making
- Status/verification commands

## Error Handling

The step handles these error cases:
- Lotus not running
- Required addresses not in context
- Contract addresses not found
- Provider registration failure
- Transaction failures
- Provider ID query failures

If already registered (provider ID file exists), the step skips execution gracefully.

## Testing

To test the step:

```bash
# Start with fresh state
cargo run -- start --reset

# Check provider ID file
cat ~/.foc-localnet/state/pdp_sp_0.provider_id.json

# Verify on-chain (inside Lotus container)
cast call <REGISTRY_PROXY> "getProviderByAddress(address)(uint256,address,address,string,bool)" \
  <PDP_SP_0_ETH_ADDRESS> --rpc-url http://localhost:1234/rpc/v1

# Verify approved provider
cast call <WARM_STORAGE_PROXY> "approvedProviders(uint256)(bool)" \
  <PROVIDER_ID> --rpc-url http://localhost:1234/rpc/v1
```

## Future Enhancements

1. Add provider registration to `status` command output
2. Create a `register-provider` subcommand for additional providers
3. Add provider deregistration support
4. Add configuration options for PDP parameters (min/max piece size, pricing, etc.)
5. Support for multiple service providers
