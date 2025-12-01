#!/bin/bash
# Setup script for MockUSDFC Foundry project
#
# This script initializes the Foundry project by installing dependencies.
# Run this once before using the Makefile targets.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Setting up MockUSDFC Foundry project..."
echo ""

# Check if forge is installed
if ! command -v forge &> /dev/null; then
    echo "Error: Foundry (forge) is not installed."
    echo ""
    echo "Please install Foundry first:"
    echo "  curl -L https://foundry.paradigm.xyz | bash"
    echo "  foundryup"
    echo ""
    echo "Or run this from the foc-builder Docker container which has Foundry pre-installed."
    exit 1
fi

echo "✓ Foundry is installed"
echo ""

# Install dependencies
echo "Installing OpenZeppelin contracts..."
cd "$SCRIPT_DIR"

if [ ! -d "lib/openzeppelin-contracts" ]; then
    forge install OpenZeppelin/openzeppelin-contracts@v5.0.0 --no-commit
    echo "✓ OpenZeppelin contracts installed"
else
    echo "✓ OpenZeppelin contracts already installed"
fi

# Install forge-std if not present
if [ ! -d "lib/forge-std" ]; then
    echo "Installing forge-std..."
    forge install foundry-rs/forge-std --no-commit
    echo "✓ forge-std installed"
else
    echo "✓ forge-std already installed"
fi

echo ""
echo "Building contracts..."
forge build

echo ""
echo "✓ Setup complete!"
echo ""
echo "You can now deploy and verify contracts using:"
echo "  make deploy PRIVATE_KEY=0x..."
echo "  make verify PRIVATE_KEY=0x... CONTRACT_ADDRESS=0x..."
echo "  make deploy-verify PRIVATE_KEY=0x..."
echo ""
