#!/bin/bash
# Test deployment script for MockUSDFC using foc-builder container
#
# This script tests the MockUSDFC deployment from within the foc-builder
# Docker container, which has Foundry pre-installed.
#
# Prerequisites:
# - foc-lotus must be running with FEVM enabled
# - foc-builder container must exist
# - You need a private key with sufficient FIL balance

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${CYAN}Testing MockUSDFC Deployment from foc-builder Container${NC}"
echo ""

# Check if PRIVATE_KEY is provided
if [ -z "$PRIVATE_KEY" ]; then
    echo -e "${RED}Error: PRIVATE_KEY environment variable is required${NC}"
    echo ""
    echo "Usage:"
    echo "  PRIVATE_KEY=0x... $0"
    exit 1
fi

# Check if foc-lotus is running
if ! docker ps --format '{{.Names}}' | grep -q '^foc-lotus$'; then
    echo -e "${RED}Error: foc-lotus container is not running${NC}"
    echo "Please start foc-localnet first: cargo run -- start"
    exit 1
fi

# Check if foc-builder exists
if ! docker ps -a --format '{{.Names}}' | grep -q '^foc-builder$'; then
    echo -e "${RED}Error: foc-builder container does not exist${NC}"
    echo "Please run 'foc-localnet init' first"
    exit 1
fi

# If foc-builder is not running, we'll use docker run instead of exec
if docker ps --format '{{.Names}}' | grep -q '^foc-builder$'; then
    DOCKER_CMD="docker exec foc-builder"
    echo -e "${GREEN}✓ foc-builder container is running${NC}"
else
    echo -e "${YELLOW}⚠ foc-builder container is not running, will use 'docker run'${NC}"
    DOCKER_CMD="docker run --rm --network host -v ${SCRIPT_DIR}:/workspace foc-builder"
fi

echo ""
echo -e "${CYAN}Step 1: Setting up Foundry project in container${NC}"

# Setup the project (install dependencies)
$DOCKER_CMD bash -c "
    cd /workspace && \
    if [ ! -d lib/openzeppelin-contracts ]; then \
        forge install OpenZeppelin/openzeppelin-contracts@v5.0.0 --no-commit; \
    fi && \
    if [ ! -d lib/forge-std ]; then \
        forge install foundry-rs/forge-std --no-commit; \
    fi && \
    forge build
"

echo -e "${GREEN}✓ Project setup complete${NC}"
echo ""

echo -e "${CYAN}Step 2: Deploying MockUSDFC${NC}"

# Deploy the contract
DEPLOY_OUTPUT=$($DOCKER_CMD bash -c "
    cd /workspace && \
    RPC_URL=http://localhost:1234/rpc/v1 \
    PRIVATE_KEY=$PRIVATE_KEY \
    forge script script/Deploy.s.sol:DeployMockUSDFC \
        --rpc-url http://localhost:1234/rpc/v1 \
        --private-key $PRIVATE_KEY \
        --broadcast \
        -vvv
" 2>&1)

echo "$DEPLOY_OUTPUT"

# Extract contract address
CONTRACT_ADDRESS=$(echo "$DEPLOY_OUTPUT" | grep "MockUSDFC deployed at:" | awk '{print $NF}')

if [ -z "$CONTRACT_ADDRESS" ]; then
    echo ""
    echo -e "${RED}Error: Failed to extract contract address from deployment output${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}✓ Contract deployed at: ${CONTRACT_ADDRESS}${NC}"
echo ""

echo -e "${CYAN}Step 3: Waiting for transaction confirmation...${NC}"
sleep 5
echo -e "${GREEN}✓ Wait complete${NC}"
echo ""

echo -e "${CYAN}Step 4: Verifying contract functions${NC}"

# Verify the contract
$DOCKER_CMD bash -c "
    cd /workspace && \
    forge script script/Verify.s.sol:VerifyMockUSDFC \
        --rpc-url http://localhost:1234/rpc/v1 \
        --private-key $PRIVATE_KEY \
        --sig 'run(address)' $CONTRACT_ADDRESS \
        -vvv
"

echo ""
echo -e "${GREEN}✓ All tests complete!${NC}"
echo ""
echo -e "${CYAN}Summary:${NC}"
echo "  Contract Address: ${CONTRACT_ADDRESS}"
echo "  RPC URL: http://localhost:1234/rpc/v1"
echo ""
