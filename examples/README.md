# FOC DevNet Examples

This directory contains examples demonstrating how to interact with a running FOC DevNet instance using the exported `devnet-info.json` file.

## Files

- `read-devnet-info.js` - JavaScript example showing how to read and use the DevNet info
- `check-balances.js` - JavaScript example demonstrating how to check user balances
- `devnet-schema.js` - Zod schema for validating DevNet info exports
- `validate-schema.js` - CLI tool for validating devnet-info.json against the schema

## Prerequisites

- Node.js 20+ installed
- A running FOC DevNet instance (`foc-devnet start`)
- npm packages: `ethers`, `zod`

## Usage

1. Start the DevNet:
   ```bash
   foc-devnet start
   ```

2. Find the devnet-info.json file:
   ```bash
   # The file is located in the run directory
   cat ~/.foc-devnet/run/<RUN_ID>/devnet-info.json
   ```

3. Install dependencies and run examples:
   ```bash
   cd examples
   npm install
   
   # Validate the schema
   npm run validate-schema ~/.foc-devnet/run/<RUN_ID>/devnet-info.json
   
   # Read DevNet info
   npm run read-info ~/.foc-devnet/run/<RUN_ID>/devnet-info.json
   
   # Check balances
   npm run check-balances ~/.foc-devnet/run/<RUN_ID>/devnet-info.json
   ```

## Available Scripts

- `npm run lint` - Check code for linting issues
- `npm run lint:fix` - Auto-fix linting issues
- `npm run read-info` - Read and display DevNet info
- `npm run check-balances` - Check user account balances

## DevNet Info Schema (Version 1)

The `devnet-info.json` file contains:

```json
{
  "version": 1,
  "info": {
    "run_id": "20260129T1219_SassyPika",
    "start_time": "2026-01-29T06:57:37.473094+00:00",
    "startup_duration": "462.21s",
    "users": [
      {
        "name": "USER_1",
        "evm_addr": "0x...",
        "native_addr": "t410f...",
        "private_key_hex": "0x..."
      }
    ],
    "contracts": {
      "multicall3_addr": "0x...",
      "mockusdfc_addr": "0x...",
      "fwss_service_proxy_addr": "0x...",
      ...
    },
    "lotus": {
      "host_rpc_url": "http://localhost:5701/rpc/v1",
      "container_id": "...",
      "container_name": "foc-..."
    },
    "lotus_miner": {
      "container_id": "...",
      "container_name": "foc-...",
      "api_port": 5703
    },
    "pdp_sps": [
      {
        "provider_id": 1,
        "eth_addr": "0x...",
        "native_addr": "t410f...",
        "pdp_service_url": "http://localhost:5714",
        "container_id": "...",
        "container_name": "foc-...",
        "is_approved": true,
        "yugabyte": {
          "web_ui_url": "http://localhost:5710",
          "master_rpc_port": 5706,
          "ysql_port": 5704
        }
      }
    ]
  }
}
```

See `read-devnet-info.js` for detailed usage of each field.
