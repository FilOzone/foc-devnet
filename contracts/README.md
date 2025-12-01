# MockUSDFC Token

A simple ERC-20 token for local development and testing with FOC (Filecoin Onchain Contracts).

## Overview

MockUSDFC is a toy implementation of the USDFC (USD Filecoin) token used in production FOC deployments. It provides the same ERC-20 interface but is designed for local testing only.

## Contract Details

- **Name**: Mock USD Filecoin
- **Symbol**: USDFC
- **Decimals**: 18
- **Initial Supply**: 1,000,000 USDFC (configurable)
- **Features**:
  - Standard ERC-20 interface (transfer, approve, transferFrom)
  - Mintable by owner
  - Simple access control (owner can mint)

## Usage in foc-localnet

The MockUSDFC token is automatically deployed during the FOC deployment step when starting the local network:

```bash
cargo run start
```

The deployment sequence:
1. GLOBAL_FIL_FAUCET → FEVM_FAUCET → FOC_DEPLOYER (fund transfers)
2. Deploy MockUSDFC using FOC_DEPLOYER account
3. Deploy FOC contracts with MockUSDFC address

## Manual Deployment

If you need to deploy the token manually:

```bash
export ETH_RPC_URL="http://localhost:1234/rpc/v1"
export DEPLOYER_ADDRESS="0x..."
export INITIAL_SUPPLY="1000000000000000000000000"  # 1 million tokens

./scripts/deploy-mock-usdfc.sh
```

## Contract Source

The contract is located at `contracts/MockUSDFC.sol` and is a minimal ERC-20 implementation for testing purposes.

**Warning**: This is a TOY contract for local development only. Do NOT use in production.

## Interacting with the Token

Once deployed, you can interact with MockUSDFC using standard ERC-20 methods:

```bash
# Check balance
cast call $TOKEN_ADDRESS "balanceOf(address)(uint256)" $USER_ADDRESS --rpc-url $ETH_RPC_URL

# Transfer tokens
cast send $TOKEN_ADDRESS "transfer(address,uint256)" $TO_ADDRESS $AMOUNT --rpc-url $ETH_RPC_URL

# Mint more tokens (owner only)
cast send $TOKEN_ADDRESS "mint(address,uint256)" $TO_ADDRESS $AMOUNT --rpc-url $ETH_RPC_URL
```

## For Production

In production deployments on Calibnet or Mainnet, the real USDFC token addresses are:
- **Calibnet**: `0xb3042734b608a1B16e9e86B374A3f3e389B4cDf0`
- **Mainnet**: `0x80B98d3aa09ffff255c3ba4A241111Ff1262F045`

These addresses are automatically used by the FOC deployment script when deploying to non-local networks.
