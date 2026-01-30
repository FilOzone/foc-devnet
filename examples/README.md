# FOC DevNet Examples

This directory contains examples demonstrating how to interact with a running FOC DevNet instance using the exported `devnet-info.json` file.

## Files

- [read-devnet-info.js](read-devnet-info.js) - JavaScript example showing how to read and use the DevNet info
- [check-balances.js](check-balances.js) - JavaScript example demonstrating how to check user balances
- [devnet-schema.js](devnet-schema.js) - Zod schema for validating DevNet info exports
- [validate-schema.js](validate-schema.js) - CLI tool for validating devnet-info.json against the schema

## Prerequisites

- Node.js 20+ installed
- A running FOC DevNet instance (`foc-devnet start`)
- npm packages: `viem`, `zod`

## Usage

1. Start the DevNet:
   ```bash
   foc-devnet start
   ```

2. Find the devnet-info.json file:
   ```bash
   # The file is located in the run directory
   cat ~/.foc-devnet/state/latest/devnet-info.json
   ```

3. Install dependencies and run examples:
   ```bash
   cd examples
   npm install
   
   # Validate the schema
   npm run validate-schema ~/.foc-devnet/state/latest/devnet-info.json
   
   # Read DevNet info
   npm run read-info ~/.foc-devnet/state/latest/devnet-info.json
   
   # Check balances
   npm run check-balances ~/.foc-devnet/state/latest/devnet-info.json
   ```
