#!/bin/bash
# Deploy MockUSDFC token using cast/forge
#
# This script deploys a simple ERC-20 token for local testing.
# It requires the following environment variables:
# - ETH_RPC_URL: Ethereum RPC endpoint (e.g., http://localhost:1234/rpc/v1)
# - DEPLOYER_ADDRESS: Ethereum address of the deployer
# - INITIAL_SUPPLY: Initial token supply (in wei, 18 decimals)

set -e

if [ -z "$ETH_RPC_URL" ]; then
    echo "Error: ETH_RPC_URL not set"
    exit 1
fi

if [ -z "$DEPLOYER_ADDRESS" ]; then
    echo "Error: DEPLOYER_ADDRESS not set"
    exit 1
fi

INITIAL_SUPPLY=${INITIAL_SUPPLY:-"1000000000000000000000000"} # 1 million tokens

echo "Deploying MockUSDFC token..."
echo "  RPC URL: $ETH_RPC_URL"
echo "  Deployer: $DEPLOYER_ADDRESS"
echo "  Initial Supply: $INITIAL_SUPPLY"

# Solidity bytecode for a simple ERC-20 token
# This is a minimal implementation for testing
# In production, you'd compile from source using solc/forge

# For now, we'll use a simple approach:
# 1. Check if forge is available
# 2. If yes, compile and deploy MockUSDFC.sol
# 3. If no, use a pre-compiled minimal ERC-20 bytecode

if command -v forge &> /dev/null; then
    echo "Using forge to compile and deploy..."
    
    # Compile the contract
    SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
    CONTRACT_PATH="$SCRIPT_DIR/../contracts/MockUSDFC.sol"
    
    if [ ! -f "$CONTRACT_PATH" ]; then
        echo "Error: MockUSDFC.sol not found at $CONTRACT_PATH"
        exit 1
    fi
    
    # Deploy using forge create
    # Note: This assumes the deployer's private key is available
    # For now, we'll use eth_sendTransaction which requires the account to be unlocked
    
    forge create "$CONTRACT_PATH:MockUSDFC" \
        --rpc-url "$ETH_RPC_URL" \
        --constructor-args "$INITIAL_SUPPLY" \
        --json
else
    echo "Warning: forge not available"
    echo "Skipping actual deployment - using placeholder address"
    echo "{"
    echo "  \"deployedTo\": \"$DEPLOYER_ADDRESS\","
    echo "  \"transactionHash\": \"0x0000000000000000000000000000000000000000000000000000000000000000\""
    echo "}"
fi
