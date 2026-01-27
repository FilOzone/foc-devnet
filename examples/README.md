# FOC DevNet Examples

This directory contains examples demonstrating how to interact with a running FOC DevNet instance using the exported `devnet-info.json` file.

## Files

- `read-devnet-info.js` - JavaScript example showing how to read and use the DevNet info
- `check-balances.js` - JavaScript example demonstrating how to check user balances

## Prerequisites

- Node.js 18+ installed
- A running FOC DevNet instance (`foc-devnet start`)
- npm packages: `ethers` (for blockchain interaction)

## Usage

1. Start the DevNet:
   ```bash
   foc-devnet start
   ```

2. Find the devnet-info.json file:
   ```bash
   # The file is located in the run directory, accessible via the latest symlink
   cat ~/.foc-devnet/state/latest/devnet-info.json
   ```

3. Run an example:
   ```bash
   cd examples
   npm install
   node read-devnet-info.js ~/.foc-devnet/state/latest/devnet-info.json
   ```

## DevNet Info Schema (Version 1)

The `devnet-info.json` file contains:

```json
{
  "version": 1,
  "info": {
    "run_id": "...",
    "start_time": "2026-01-27T...",
    "startup_duration": "539.04s",
    "users": [...],
    "contracts": {...},
    "lotus": {...},
    "lotus_miner": {...},
    "curio_providers": [...]
  }
}
```

See `read-devnet-info.js` for detailed usage of each field.
