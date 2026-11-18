# FOC DevNet Examples

This directory contains examples demonstrating how to interact with a running FOC DevNet instance using the exported `devnet-info.json` file.

## Files

- [read-devnet-info.js](read-devnet-info.js) - JavaScript example showing how to read and use the DevNet info
- [check-balances.js](check-balances.js) - JavaScript example demonstrating how to check user balances
- [devnet-schema.js](devnet-schema.js) - Zod schema for validating DevNet info exports
- [validate-schema.js](validate-schema.js) - CLI tool for validating devnet-info.json against the schema

## Prerequisites

- Node.js 24+ and npm
- A running devnet started via the [quick start](../README.md)
- Access to the npm registry to install dependencies

## Usage

Run from the repository root:

```bash
DEVNET_INFO="$HOME/.foc-devnet/state/latest/devnet-info.json"
cd examples
npm install
node validate-schema.js "$DEVNET_INFO"
npm run read-info -- "$DEVNET_INFO"
npm run check-balances -- "$DEVNET_INFO"
```

With a custom `FOC_DEVNET_BASEDIR`, use its absolute path in place of
`$HOME/.foc-devnet`. For a specific run, use `<base-dir>/run/<run-id>/devnet-info.json`.
The versioned export contains raw test private keys. Do not share the file or
reuse the keys outside the devnet.

## Exercise storage

Run the repository's Synapse end-to-end scenario from the repository root.
It requires Python 3, Node.js 24+, npm registry access, and two usable storage
providers (the default configuration).

```bash
python3 scenarios/test_synapse_e2e.py
```

With a compatible version of filecoin-pin already installed, add a file with:

```bash
filecoin-pin add --network devnet path/to/file
```

For the full test suite, see [Scenario Tests](../README_ADVANCED.md#scenario-tests).
