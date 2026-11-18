# MockUSDFC Token

MockUSDFC is the ERC-20 test token for local FOC (Filecoin Onchain Contracts) development.

## Overview

**Test use only.** Any caller can mint tokens to any address or burn tokens
from any holder without approval. Do not deploy this contract in production.

## Contract Details

- **Name**: Mock USDC
- **Symbol**: USDFC
- **Decimals**: 18
- **Interface**: ERC-20 with ERC-2612 permit, plus unrestricted `mint` and `burn`

## Usage in foc-devnet

`foc-devnet start` deploys MockUSDFC for the local network as part of startup.

## Manual Deployment

See the [MockUSDFC Foundry project](MockUSDFC/README.md) for build, test, and
manual deployment commands using [Foundry](https://www.getfoundry.sh/).

## Contract Source

[`MockUSDFC/src/MockUSDFC.sol`](MockUSDFC/src/MockUSDFC.sol)
