# MockUSDFC - Foundry Project

A complete Foundry-based project for deploying and verifying the MockUSDFC token contract. This project is part of the FOC (Filecoin Onchain Contracts) localnet testing infrastructure.

## Overview

MockUSDFC is a simple ERC-20 token contract used for testing FOC warm storage services. It includes:

- **ERC-20 Token**: Standard fungible token with 18 decimals
- **Mint/Burn Functions**: Testing utilities for token supply management
- **Deployment Script**: Automated deployment via Foundry scripts
- **Verification Script**: Comprehensive testing of all contract functions

## Prerequisites

- [Foundry](https://book.getfoundry.sh/getting-started/installation) installed
- Running FOC localnet with Lotus FEVM enabled (port 1234)
- Private key for deployment account with sufficient FIL balance

## Quick Start

### 1. Install Dependencies

```bash
cd contracts/MockUSDFC
make install
```

This will install OpenZeppelin contracts v5.0.0.

### 2. Build Contracts

```bash
make build
```

### 3. Deploy Contract

```bash
make deploy PRIVATE_KEY=0x<your_private_key>
```

This will:
- Deploy MockUSDFC with 1,000,000 tokens initial supply
- Display the deployed contract address
- Save deployment details to `broadcast/` directory

### 4. Verify Contract Functions

```bash
make verify PRIVATE_KEY=0x<your_private_key> CONTRACT_ADDRESS=0x<deployed_address>
```

This will test:
- ✓ Token metadata (name, symbol, decimals)
- ✓ Balance queries
- ✓ Transfer functionality
- ✓ Mint functionality
- ✓ Burn functionality

### 5. Deploy and Verify in One Step

```bash
make deploy-verify PRIVATE_KEY=0x<your_private_key>
```

This convenience target will:
1. Deploy the contract
2. Wait 5 seconds for transaction confirmation
3. Automatically verify all functions

## Environment Variables

All commands support these optional environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `PRIVATE_KEY` | Deployer's private key (hex format) | **Required** |
| `CONTRACT_ADDRESS` | Address of deployed contract (for verify) | **Required for verify** |
| `RPC_URL` | Filecoin FEVM RPC endpoint | `http://localhost:1234/rpc/v1` |
| `INITIAL_SUPPLY` | Initial token supply (in wei) | `1000000000000000000000000` (1M tokens) |

### Examples

**Custom RPC URL:**
```bash
make deploy PRIVATE_KEY=0xabc123... RPC_URL=http://custom-rpc:1234/rpc/v1
```

**Custom Initial Supply (10M tokens):**
```bash
make deploy PRIVATE_KEY=0xabc123... INITIAL_SUPPLY=10000000000000000000000000
```

## Manual Deployment (Without Makefile)

If you prefer to use Foundry commands directly:

### Deploy

```bash
forge script script/Deploy.s.sol:DeployMockUSDFC \
  --rpc-url http://localhost:1234/rpc/v1 \
  --private-key 0x<your_private_key> \
  --broadcast \
  -vvv
```

### Verify

```bash
forge script script/Verify.s.sol:VerifyMockUSDFC \
  --rpc-url http://localhost:1234/rpc/v1 \
  --private-key 0x<your_private_key> \
  --sig "run(address)" 0x<contract_address> \
  -vvv
```

## Contract Functions

### Standard ERC-20 Functions

- `name()` → `"Mock USDC"`
- `symbol()` → `"USDFC"`
- `decimals()` → `18`
- `totalSupply()` → Current total supply
- `balanceOf(address)` → Balance of address
- `transfer(address, uint256)` → Transfer tokens
- `approve(address, uint256)` → Approve spending
- `transferFrom(address, address, uint256)` → Transfer on behalf

### Testing Functions

- `mint(address to, uint256 amount)` → Mint new tokens (unrestricted)
- `burn(address from, uint256 amount)` → Burn tokens (unrestricted)

⚠️ **Note**: Mint and burn functions are intentionally unrestricted for testing purposes. Do not use in production.

## Integration with foc-localnet

This Foundry project is designed to simplify the deployment logic in `src/commands/start/usdfc_deploy.rs`. Instead of manually constructing deployment transactions in Rust, the deployment step can now:

1. **From Host**: Run `make deploy-verify` directly during development
2. **From Rust**: Execute the Makefile targets via `Command::new("make")`
3. **From foc-builder**: Mount this directory and run Foundry scripts

### Example Rust Integration

```rust
// In usdfc_deploy.rs
let output = Command::new("make")
    .current_dir("/path/to/contracts/MockUSDFC")
    .arg("deploy-verify")
    .env("PRIVATE_KEY", deployer_private_key)
    .env("RPC_URL", "http://localhost:1234/rpc/v1")
    .output()?;

// Parse contract address from output
let contract_address = extract_contract_address(&output.stdout)?;
```

## Project Structure

```
MockUSDFC/
├── foundry.toml           # Foundry configuration
├── Makefile              # Convenient deployment commands
├── README.md             # This file
├── src/
│   └── MockUSDFC.sol     # Token contract
├── script/
│   ├── Deploy.s.sol      # Deployment script
│   └── Verify.s.sol      # Verification script
├── lib/                  # Dependencies (OpenZeppelin)
├── out/                  # Compiled artifacts (gitignored)
├── cache/                # Build cache (gitignored)
└── broadcast/            # Deployment logs
```

## Troubleshooting

### "Error: PRIVATE_KEY is required"

Make sure to provide the private key in hex format:
```bash
make deploy PRIVATE_KEY=0xabc123...
```

### "Failed to extract contract address"

The deployment may have failed. Check the output for errors. Common issues:
- Insufficient FIL balance for gas
- FEVM not enabled on Lotus
- RPC endpoint not accessible

### "Transaction reverted"

Wait a few more seconds after deployment before verifying:
```bash
make deploy PRIVATE_KEY=0x...
sleep 10  # Wait longer
make verify PRIVATE_KEY=0x... CONTRACT_ADDRESS=0x...
```

### OpenZeppelin Import Errors

Reinstall dependencies:
```bash
make clean
make install
make build
```

## Development

### Run Tests

```bash
make test
```

### Clean Build Artifacts

```bash
make clean
```

### View All Available Commands

```bash
make help
```

## License

MIT License - See LICENSE file in repository root.
