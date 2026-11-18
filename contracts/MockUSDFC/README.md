# MockUSDFC - Foundry Project

Use this project to build, test, and manually deploy the
[MockUSDFC test token](../README.md). foc-devnet deploys its own instance during
startup; the manual commands below are not required for a normal startup.

## Prerequisites

- [Foundry](https://www.getfoundry.sh/introduction/installation), Git, Make,
  and Bash for the commands below.
- For deployment and verification only: a running local FEVM RPC endpoint and
  a funded test account's hex private key. Never use a production key.

## Quick Start

### 1. Install Dependencies

```bash
cd contracts/MockUSDFC
forge install --root "$PWD" --no-git OpenZeppelin/openzeppelin-contracts@v5.0.0 \
  foundry-rs/forge-std@v1.12.0
```

The scripts and tests need `forge-std` as well as OpenZeppelin Contracts.
Use the command above rather than `make install`, which only requests
OpenZeppelin and uses an older Forge option.

### 2. Build and Test

```bash
make build
make test
```

### 3. Deploy Contract

For a foc-devnet run, use `info.lotus.host_rpc_url` from `devnet-info.json`
instead of assuming a fixed port. See [Accessing Lotus API](../../README_ADVANCED.md#accessing-lotus-api).
Set the URL once so deployment and verification target the same endpoint:

```bash
read -rp 'Local FEVM RPC URL: ' RPC_URL
export RPC_URL
read -rsp 'Local test account private key (0x-prefixed): ' PRIVATE_KEY
printf '\n'
make deploy PRIVATE_KEY="$PRIVATE_KEY"
```

`make deploy` uses `--broadcast` and sends a transaction to `RPC_URL`. Once
Foundry reports a successful broadcast, use the logged address or the
deployment record in `broadcast/`. A script log alone does not prove that the
transaction succeeded.

### 4. Verify Contract Functions

```bash
read -rp 'Deployed MockUSDFC address: ' CONTRACT_ADDRESS
make verify PRIVATE_KEY="$PRIVATE_KEY" CONTRACT_ADDRESS="$CONTRACT_ADDRESS"
```

This simulates the checks against the deployed contract using the RPC state;
it does not broadcast or change live balances. It checks metadata and
exercises mint and burn. The simulated transfer runs only if the signer holds
at least 1,000 tokens. The checks expect the fixed mint recipient (and the
fixed transfer recipient, if exercised) to have no existing token balance.
Prefer a fresh deployment.

### 5. Deploy and Verify in One Step

```bash
make deploy-verify PRIVATE_KEY="$PRIVATE_KEY"
```

This target broadcasts the deployment, extracts the logged address, and waits
five seconds before running the same simulation. Check Foundry's deployment
result if the follow-up check fails.

## Environment Variables

Deployment and verification accept the following settings:

| Variable | Description | Default |
|----------|-------------|---------|
| `PRIVATE_KEY` | Funded local test account's hex key | Required |
| `CONTRACT_ADDRESS` | Deployed token address | Required for `make verify` |
| `RPC_URL` | Local FEVM RPC endpoint | `http://localhost:1234/rpc/v1` |
| `INITIAL_SUPPLY` | Deployment supply in base units (18 decimals) | `1000000000000000000000000` (1 million tokens) |

### Examples

**Custom Initial Supply (10M tokens):**
```bash
INITIAL_SUPPLY=10000000000000000000000000 make deploy PRIVATE_KEY="$PRIVATE_KEY"
```

## Manual Deployment (Without Makefile)

To call the scripts directly, use the same key and contract address as above:

### Deploy

```bash
forge script script/Deploy.s.sol:DeployMockUSDFC \
  --rpc-url "${RPC_URL:-http://localhost:1234/rpc/v1}" \
  --private-key "$PRIVATE_KEY" \
  --broadcast \
  --slow \
  --gas-estimate-multiplier 10000 \
  -vvv
```

### Verify

```bash
forge script script/Verify.s.sol:VerifyMockUSDFC \
  --rpc-url "${RPC_URL:-http://localhost:1234/rpc/v1}" \
  --private-key "$PRIVATE_KEY" \
  --sig 'run(address)' "$CONTRACT_ADDRESS" \
  -vvv
```

## License

MIT License — see the [repository license](../../LICENSE).
