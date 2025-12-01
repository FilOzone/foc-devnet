# FOC Deployment Guide

This document explains the Filecoin Onchain Contracts (FOC) deployment process in foc-localnet.

## Overview

The FOC deployment step (`FOCDeploy`) is executed automatically when starting the local cluster. It deploys all necessary smart contracts for the FOC warm storage service, including a mock USDFC token for local testing.

## Deployment Sequence

The FOC deployment happens in this order during `cargo run start`:

1. **Lotus** - Starts with FEVM enabled
2. **Lotus-Miner** - Builds blocks and tipsets
3. **FOCDeploy** - **← Deploys FOC contracts (NEW)**
4. **Yugabyte** - Database for Curio
5. **Curio** - Second generation miner (uses FOC contracts)

## Fund Transfer Chain

```
GLOBAL_FIL_FAUCET (50,000 FIL)
    ↓ 10,000 FIL
FEVM_FAUCET (f4 address)
    ↓ 5,000 FIL
FOC_DEPLOYER (f4 address)
    ↓ Deploys contracts
MockUSDFC + FOC Contracts
```

### Account Details

- **GLOBAL_FIL_FAUCET**: Pre-funded BLS account from genesis (`prefunded-1`)
- **FEVM_FAUCET**: f4 (delegated) address for FEVM ecosystem operations
- **FOC_DEPLOYER**: f4 address that actually deploys all contracts

## Deployed Contracts

### MockUSDFC Token
- **Purpose**: Toy ERC-20 token for local testing (replaces production USDFC)
- **Symbol**: USDFC
- **Decimals**: 18
- **Initial Supply**: 1,000,000 tokens
- **Contract**: `contracts/MockUSDFC.sol`

### FOC Service Contracts
All contracts are deployed using `deploy-all-warm-storage.sh` from filecoin-services:

1. **PDPVerifier Implementation** - Proof of Data Possession verifier
2. **PDPVerifier Proxy** - Upgradeable proxy for PDPVerifier
3. **FilecoinPayV1 Contract** - Payment handling
4. **ServiceProviderRegistry Implementation** - Registry for service providers
5. **ServiceProviderRegistry Proxy** - Upgradeable proxy
6. **SignatureVerificationLib** - Library for signature verification
7. **FilecoinWarmStorageService Implementation** - Main warm storage service
8. **FilecoinWarmStorageService Proxy** - Upgradeable proxy
9. **FilecoinWarmStorageServiceStateView** - View contract for reading state

## Contract Addresses

After deployment, all contract addresses are saved to:

```
~/.foc-localnet/artifacts/docker/volumes/foc-contract-addresses.json
```

Example structure:
```json
{
  "MockUSDFC": "0x...",
  "PDPVerifier Implementation": "0x...",
  "PDPVerifier Proxy": "0x...",
  "FilecoinPayV1 Contract": "0x...",
  "ServiceProviderRegistry Implementation": "0x...",
  "ServiceProviderRegistry Proxy": "0x...",
  "FilecoinWarmStorageService Implementation": "0x...",
  "FilecoinWarmStorageService Proxy": "0x...",
  "FilecoinWarmStorageServiceStateView": "0x..."
}
```

## Configuration

The deployment uses these environment variables:

- `ETH_RPC_URL`: http://localhost:1234/rpc/v1 (Lotus FEVM endpoint)
- `USDFC_TOKEN_ADDRESS`: Address of deployed MockUSDFC
- `SERVICE_NAME`: "FOC LocalNet Warm Storage"
- `SERVICE_DESCRIPTION`: "Warm storage service for FOC local development network"
- `DRY_RUN`: false (actual deployment, not simulation)
- `CHAIN`: 31415926 (local network chain ID)
- `AUTO_VERIFY`: false (skip contract verification on block explorers)

## Using Deployed Contracts

### For Curio Integration

Curio will need the contract addresses from `foc-contract-addresses.json`. Key contracts:

- **FilecoinWarmStorageService Proxy**: Main service contract
- **ServiceProviderRegistry Proxy**: Register as a storage provider
- **MockUSDFC**: Token for payment operations

### Interacting with Contracts

Using `cast` from Foundry:

```bash
# Read contract state
cast call <CONTRACT_ADDRESS> "functionName()(returnType)" --rpc-url http://localhost:1234/rpc/v1

# Send transaction
cast send <CONTRACT_ADDRESS> "functionName(args)" <args> --rpc-url http://localhost:1234/rpc/v1 --unlocked --from <ETH_ADDRESS>

# Check USDFC balance
cast call <MOCK_USDFC_ADDRESS> "balanceOf(address)(uint256)" <ETH_ADDRESS> --rpc-url http://localhost:1234/rpc/v1
```

## Troubleshooting

### Deployment Fails

1. **Check Lotus is running with FEVM**:
   ```bash
   docker ps | grep foc-lotus
   docker logs foc-lotus | grep -i "fevm\|eth"
   ```

2. **Verify FOC_DEPLOYER has funds**:
   ```bash
   docker exec foc-lotus /usr/local/bin/lotus-bins/lotus wallet balance <FOC_DEPLOYER_ADDRESS>
   ```

3. **Check deployment logs**:
   Look for detailed output in the FOCDeploy step output during `cargo run start`

4. **Verify foc-builder has Foundry**:
   ```bash
   docker run --rm foc-builder forge --version
   docker run --rm foc-builder cast --version
   ```

### Contract Address Not Found

If `foc-contract-addresses.json` is missing or empty:

1. Check if FOCDeploy step completed successfully
2. Look for errors in deployment output
3. Verify the deployment script ran completely
4. Check file permissions on volumes directory

### MockUSDFC Deployment Fails

If MockUSDFC fails to deploy:

1. Verify `contracts/MockUSDFC.sol` exists
2. Check FOC_DEPLOYER has sufficient FIL
3. Ensure FEVM is enabled in Lotus
4. Check Ethereum RPC is accessible at http://localhost:1234/rpc/v1

## Development vs Production

### Local Development (foc-localnet)
- Uses MockUSDFC token
- Chain ID: 31415926 (local)
- All contracts deployed fresh on each start
- No contract verification on block explorers

### Production (Calibnet/Mainnet)
- Uses real USDFC token:
  - Calibnet: `0xb3042734b608a1B16e9e86B374A3f3e389B4cDf0`
  - Mainnet: `0x80B98d3aa09ffff255c3ba4A241111Ff1262F045`
- Chain ID: 314159 (Calibnet) or 314 (Mainnet)
- Contracts deployed once and reused
- Contracts verified on block explorers

## References

- FOC Repository: https://github.com/FilOzone/filecoin-services
- Deployment Script: `filecoin-services/service_contracts/tools/deploy-all-warm-storage.sh`
- Lotus FEVM Docs: https://lotus.filecoin.io/lotus/developers/local-network/#fevm-features
- Foundry Docs: https://book.getfoundry.sh/
