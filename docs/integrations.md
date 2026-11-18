# Inspect a running devnet

For application integration and export validation, see the [client examples](../examples/README.md).

### Accessing Lotus API
Use the exported URL instead of assuming a host port. The devnet Lotus API
does not require authentication; keep it on a trusted development network.
This example uses `jq` and `curl`:

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
info="$base/state/latest/devnet-info.json"
rpc_url=$(jq -er '.info.lotus.host_rpc_url' "$info")
curl -fsS "$rpc_url" \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"Filecoin.ChainHead","params":[],"id":1}'
```

### Contract Interaction
Use [Foundry's `cast`](https://getfoundry.sh/) on the host with the exported
host RPC URL. For example, read the MockUSDFC supply from a running devnet:

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
info="$base/state/latest/devnet-info.json"
rpc_url=$(jq -er '.info.lotus.host_rpc_url' "$info")
token_addr=$(jq -er '.info.contracts.mockusdfc_addr' "$info")
cast call --rpc-url "$rpc_url" "$token_addr" 'totalSupply()(uint256)'
```
