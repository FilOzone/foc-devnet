# Service provider workflows

Use these recipes with the [provider settings](configuration.md#configuration-parameters) in `config.toml`.

## Service Provider Examples

### Example 1: Run 1 SP with 0 Authorized

To test behavior with a registered but unapproved provider, set these
top-level entries in the existing `config.toml`:

```toml
active_pdp_sp_count = 1
approved_pdp_sp_count = 0
endorsed_pdp_sp_count = 0
```

Run `foc-devnet start` and [inspect the export](#querying-sp-status). The only
provider should have both `is_approved` and `is_endorsed` set to `false`.

### Example 2: Run 3 SPs with Top 2 Authorized

To test a mix of approved, endorsed, and unapproved providers, set:

```toml
active_pdp_sp_count = 3
approved_pdp_sp_count = 2
endorsed_pdp_sp_count = 1
```

Run `foc-devnet start` and [inspect the export](#querying-sp-status). Providers
1 and 2 are approved in Warm Storage; only provider 1 is endorsed. Provider 3
is registered but neither approved nor endorsed.

### Example 3: Maximum SPs (5)

To run the maximum number of providers and approve all of them, set:

```toml
active_pdp_sp_count = 5
approved_pdp_sp_count = 5
endorsed_pdp_sp_count = 5
```

Run `foc-devnet start` and [inspect the export](#querying-sp-status). Each
provider has its own Postgres and Scylla containers and its own Curio network.

### Querying SP Status

Inspect the approval and endorsement flags recorded at startup, and the
registry proxy address, with `jq`:

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
info="$base/state/latest/devnet-info.json"
jq '.info.pdp_sps | map({provider_id, container_name, pdp_service_url, is_approved, is_endorsed})' "$info"
jq -er '.info.contracts.service_provider_registry_proxy_addr' "$info"
```

Approval is in the Warm Storage contract; endorsement is tracked separately.
The export's `provider_id` is the one-based instance index. To look up the
actual on-chain ID for instance `N`, use `pdp_sps/N.provider_id.json` in the
run directory.

### Querying the Databases

Each provider has its own Postgres and Scylla containers. Postgres uses `curio`
as the user, password, and database name. To inspect schemas for provider 1
on a running devnet, use `jq` and Docker:

```bash
(
  base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
  run_id=$(jq -er '.run_id' "$base/state/current_runid.json") || exit 1
  docker exec "foc-$run_id-postgres-1" psql -U curio -d curio -c '\dn'
  docker exec "foc-$run_id-scylla-1" cqlsh -e 'DESCRIBE KEYSPACES;'
)
```
