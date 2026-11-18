# Advanced Guide: foc-devnet

This guide covers configuration, lifecycle, and troubleshooting for a local foc-devnet.

---

## Commands Reference

### `clean`
Removes the contents of the selected [base directory](#foc_devnet_basedir),
including builds, keys, cached proof parameters, run data, and logs. It keeps
the directory itself and preserves `config.toml` by default. **Run
[`stop`](#stop) first**: `clean` deletes local state but does not stop
containers or remove Docker networks.

```bash
foc-devnet clean [OPTIONS]
```

**Options:**
- `--all` — Also remove `config.toml`.
- `--images` — Also attempt to remove the shared `foc-lotus`, `foc-lotus-miner`,
  `foc-curio`, and `foc-builder` Docker images. Stock images, unrelated images,
  and Portainer's container and data volume are not removed.

For example, to remove local data and foc-devnet images while keeping the
configuration for the next `init`:

```bash
foc-devnet stop && foc-devnet clean --images
```

### `init`
Initializes the base directory, prepares dependency repositories, and builds
Docker images. It does not build the Lotus or Curio binaries.

Use a fresh base directory or run [`stop`](#stop) followed by [`clean`](#clean)
before reinitializing. If `config.toml` already exists, `init` reuses it and
applies any explicit source overrides.

```bash
foc-devnet init [OPTIONS]
```

**Options:**
- `--curio <SOURCE>` - Curio source location
- `--lotus <SOURCE>` - Lotus source location
- `--filecoin-services <SOURCE>` - Filecoin Services source location
- `--pdp <SOURCE>` — Override the PDP source. If `pdp` is not set in the saved configuration and no override is supplied, use the PDP submodule bundled with Filecoin Services.
- `--proof-params-dir <PATH>` — Stage proof parameters from a local directory.
- `--rand` — Generate a random mnemonic instead of the deterministic test mnemonic.
- `--no-docker-build` — Skip artifact staging (including `--proof-params-dir`) and Docker image builds. Use only when the required images are already available.

**Source Format:**
- `latesttag` — Newest version-sorted Git tag in the configured repo (resolved once at `init`).
- `latesttag:<selector>` - Newest git tag matching a glob selector, e.g. `latesttag:v*` or `latesttag:pdp/v*`.
- `latesttag:<url>:<selector>` - Newest matching tag from a custom repo, e.g. `latesttag:https://github.com/org/repo.git:v*`.
- `gittag:<tag>` — Specific tag in the configured repo.
- `gittag:<url>:<tag>` - Tag from custom repo, e.g. `gittag:https://github.com/org/repo.git:v1.0.0`
- `gitcommit:<sha>` — Specific commit in the configured repo.
- `gitcommit:<url>:<sha>` - Commit from custom repo
- `gitbranch:<branch>` — Specific branch in the configured repo.
- `gitbranch:<url>:<branch>` - Branch from custom repo
- `local:<dir>` - Local directory

Without a custom URL, Git selectors use the configured Git URL, or the built-in
URL if there is no configured Git URL for that component.

**Example:**
```bash
foc-devnet init --lotus local:/home/user/lotus
```

### `build`
Builds Lotus (including `lotus-miner`) or Curio binaries in Docker. Run both
after [`init`](#init); output binaries are placed in `bin/` under the base
directory.

```bash
foc-devnet build lotus
foc-devnet build curio
```

Both subcommands accept an optional positional path but **ignore it**. To use
a local checkout, select it during `init`, for example
`foc-devnet init --curio local:/home/user/curio`.

### `start`
Starts a fresh local Filecoin network. Run after `init`, `build lotus`, and
`build curio`. Startup first checks [host name resolution](#setup-requirements),
then attempts to [stop existing devnet services](#stop) on the same Docker daemon.

```bash
foc-devnet start [--parallel]
```

Startup is sequential by default. `--parallel` enables **experimental**
concurrent execution of independent steps; use the default when troubleshooting.
See the [startup sequence](#detailed-start-sequence) and
[run outputs](#key-files) for what to expect.

### `stop`
Stops and removes devnet service containers and attempts to remove their Docker
networks. It clears the current run ID from the selected base directory, but
preserves run files and leaves [Portainer](#portainer-your-debugging-companion)
running.

```bash
foc-devnet stop
```

**Docker-wide devnet scope:** In addition to the recorded run, `stop` sweeps
containers based on foc-devnet-built images and networks matching the devnet
naming scheme on the current Docker daemon. It is not limited to
`FOC_DEVNET_BASEDIR`. Use the same base directory as `start` so `stop` can also
find the recorded run's Postgres and Scylla containers, which use stock images.

### `status`
Shows the current status of the foc-devnet system.

```bash
foc-devnet status
```

Use it to inspect the recorded run, container states, and published ports.
The builder is not a persistent service, so it need not appear as running.

### `version`
Shows version information.

```bash
foc-devnet version [--notty]
```

`--notty` forces plain output without tracing prefixes even on a terminal.

---

## Network Architecture & Inter-SP Communication

### How It Works: `host.docker.internal`

The service registry advertises provider URLs as
`http://host.docker.internal:<published-port>` so host-side clients and other
Curio containers can both use them. On the host, this name must resolve to a
loopback address. Curio containers use Docker's `host-gateway` mapping to reach
the published ports. The host-only URL in `devnet-info.json` uses `localhost`.
Provider connections use plain HTTP; use this setup only on a trusted development network.

### Setup Requirements

Check name resolution on the host, including on Docker Desktop and in CI.
If `host.docker.internal` does not resolve to loopback, add the following entry
to the host's `/etc/hosts` (update any existing entry instead of duplicating it):

```text
127.0.0.1 host.docker.internal
```

### Validation

`foc-devnet start` checks for a loopback address for `host.docker.internal`
before stopping any existing cluster. If it fails, correct the host mapping
before retrying.

---

## Configuration System

### Config File Location

The configuration file is `~/.foc-devnet/config.toml` by default, or
`$FOC_DEVNET_BASEDIR/config.toml` when the base directory is overridden.

### Config Structure

The following is a complete example with the current compiled defaults.
Each component source has one variant table, such as `[lotus.GitTag]`.
Set top-level port and provider counts before the source tables.

```toml
port_range_start = 5700
port_range_count = 100
active_pdp_sp_count = 2
approved_pdp_sp_count = 2
endorsed_pdp_sp_count = 1

[lotus.GitTag]
url = "https://github.com/filecoin-project/lotus.git"
tag = "v1.39.1"

[curio.GitTag]
url = "https://github.com/filecoin-project/curio.git"
tag = "v1.29.2"

[filecoin_services.GitTag]
url = "https://github.com/FilOzone/filecoin-services.git"
tag = "v1.5.0"

[multicall3.GitTag]
url = "https://github.com/mds1/multicall3.git"
tag = "v3.1.0"
```

The source variants are `GitTag` (`url`, `tag`), `GitCommit` (`url`, `commit`),
`GitBranch` (`url`, `branch`), and `LocalSource` (`dir`). PDP is optional.
Without a `pdp` source table, deployment uses Filecoin Services'
`service_contracts/lib/pdp` submodule. For example, add this table to use an
existing local PDP checkout:

```toml
[pdp.LocalSource]
dir = "/home/user/dev/pdp"
```

### Configuration Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `port_range_start` | u16 | 5700 | First host port in the configured range |
| `port_range_count` | u16 | 100 | Number of host ports in the range |
| `active_pdp_sp_count` | usize | 2 | Number of Curio providers to run and register |
| `approved_pdp_sp_count` | usize | 2 | Number of those providers to approve in Warm Storage |
| `endorsed_pdp_sp_count` | usize | 1 | Number of approved providers to endorse |

Provider counts must satisfy
`0 ≤ endorsed_pdp_sp_count ≤ approved_pdp_sp_count ≤ active_pdp_sp_count ≤ 5`.
If `approved_pdp_sp_count = 0`, set `endorsed_pdp_sp_count = 0` as well.
Use a nonempty [port range](#port-management) with enough available ports for
the services configured.

### How Defaults Work

`init` writes [`Config::default()`](src/config.rs) when no configuration file
exists. An upgrade does not replace existing settings. To regenerate the
configuration, back it up first, then run:

```bash
foc-devnet stop && foc-devnet clean --all && foc-devnet init
```

This deletes local run data as well as the old configuration. Plain `clean`
preserves `config.toml` for the next `init`.

### Editing Config

Edit `config.toml` to change the ports or provider counts for the next `start`.
For source changes, use [`init` source overrides](#init) when reinitializing.
Keep ports and counts at the top level, before the component tables.

---

### Environment Variables

#### `FOC_DEVNET_BASEDIR`

Overrides the default `~/.foc-devnet` base directory. Use it to keep different
configurations and their files on separate paths, or to put the data on another
disk. It does **not** isolate Docker resources. Use only one running foc-devnet
per Docker daemon: [start and stop share cleanup scope](#stop), startup
replaces an existing devnet Portainer, and `clean --images` acts on shared images.

Use an absolute path for shell and scenario commands. For example, after
stopping any previous devnet from its original base directory:

```bash
export FOC_DEVNET_BASEDIR="$HOME/foc-test-env"
foc-devnet init
foc-devnet build lotus
foc-devnet build curio
foc-devnet start
```

The CLI expands a leading `~`, but not every helper does. The shell examples
below assume `FOC_DEVNET_BASEDIR` is unset or already expanded to an absolute
path.

---

## Directory Structure

```
<base-dir>/
├── config.toml
├── bin/                             # Compiled binaries
├── code/                            # Dependency checkouts or local source symlinks
├── docker/volumes/
│   ├── cache/                       # Shared proof parameters
│   └── run-specific/<run-id>/       # Data for one run
│       ├── genesis/
│       ├── lotus-data/               # Includes the Lotus API token
│       ├── lotus-miner-data/
│       └── curio/                    # One directory per SP, numbered from 1
│           ├── 1/
│           └── 2/
├── keys/                            # Test keys and mnemonic; do not share
├── logs/                            # Includes build logs under build/
├── run/<run-id>/
│   ├── setup.log
│   ├── logs/                        # Container logs captured at startup finalization
│   ├── post_start_status.log
│   ├── version.txt
│   ├── devnet-info.json              # Versioned export for consumers
│   └── pdp_sps/*.provider_id.json
├── state/
│   ├── current_run_id.txt           # Cleared by stop
│   └── latest                       # Symlink to latest recorded run directory
└── tmp/
```

### Key Files

Use `devnet-info.json` for scripts and SDKs. The export includes a top-level
`version` and an `info` object with the run ID, host RPC URL, contract addresses,
users and their **test private keys**, and provider details. See the
[schema](examples/devnet-schema.js) and [validation instructions](examples/README.md).
Do not publish the file or use its keys outside the devnet.

Internal artifacts may change independently of the export. For example,
[`contract_addresses.json`](src/commands/start/foc_deploy/contract_addresses.rs)
contains separate `contracts` and `foc_contracts` maps, not a flat contract list.

---

## Resetting the System

### Normal Start Behavior

Each `start` attempts to stop the existing devnet and creates a new run with a
fresh chain. It preserves older run directories and their run-specific data on
the host. A `stop` followed by `start` does not resume the old chain.

`state/latest` is updated during startup, **before** there is an export. It is
not a pointer to the last successful or currently running devnet. `stop` leaves
the link in place. If startup fails, the new run may lack `devnet-info.json`.
Even after a successful startup, an export failure is logged as a warning.
Check the selected run and export before using its endpoints. Older exports
under `run/<run-id>/` are useful as records; they do not keep the services live.

### Manual Cleanup

Use [`clean`](#clean) after stopping if none of the local runs need to be kept.
To keep the currently recorded run but remove other historical runs and their
data volumes, run the following in Bash while that run is still recorded. It
validates and captures the ID **before** `stop` clears it, previews only matching
run directories under the two data roots, and asks before deleting. Do not run
`start` or change the base directory during cleanup.

```bash
(
  set -e
  base=$(cd -- "${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}" && pwd -P)
  export FOC_DEVNET_BASEDIR="$base"
  keep=$(cat "$base/state/current_run_id.txt")
  run_id_pattern='^[0-9]{8}T[0-9]{4}_[A-Za-z]+$'
  if [[ ! $keep =~ $run_id_pattern || ! -d "$base/run/$keep" ||
        ! "$base/state/latest" -ef "$base/run/$keep" ]]; then
    printf 'Cannot confirm the run to keep; no data removed.\n' >&2
    exit 1
  fi
  roots=("$base/run" "$base/docker/volumes/run-specific")
  for root in "${roots[@]}"; do
    if [[ $(cd -- "$root" && pwd -P) != "$root" ]]; then
      printf 'Unexpected run directory: %s; no data removed.\n' "$root" >&2
      exit 1
    fi
  done

  foc-devnet stop
  old_runs=()
  for root in "${roots[@]}"; do
    for dir in "$root"/*; do
      [[ -d $dir && ! -L $dir ]] || continue
      id=${dir##*/}
      [[ $id =~ $run_id_pattern && $id != "$keep" ]] || continue
      old_runs+=("$dir")
    done
  done
  if ((${#old_runs[@]} == 0)); then
    printf 'No historical run directories to remove.\n'
    exit 0
  fi
  printf 'Keep run: %s\nRemove:\n' "$keep"
  printf '  %s\n' "${old_runs[@]}"
  read -r -p 'Type yes to remove these directories: ' answer
  if [[ $answer == yes ]]; then
    rm -rf -- "${old_runs[@]}"
  fi
)
```

---

## Run ID and Step Context

### Run ID

Run IDs have the form `YYYYMMDDTHHMM_RandomName`, for example
`20260102T1430_ZanyPip`. They identify a run's Docker resources and files but
do not make concurrent runs safe. See [Normal Start Behavior](#normal-start-behavior)
for the distinction between the current ID and the latest-run link.

### Step Context (SetupContext)

`step_context.json` is internal startup diagnostics. Its keys and contents
are defined by [the startup steps](src/commands/start/), not by the exported
schema. Use [`devnet-info.json`](#key-files) for integrations.

---

## Docker and Networking

Docker runs the devnet services and the Lotus and Curio builds. See the
[installation prerequisites](README.md#prerequisites) for the host requirements.

### Portainer: Your Debugging Companion

[Portainer](https://docs.portainer.io/) provides live inspection of Docker
containers, networks, and logs. Open `http://localhost:5700` with the default
configuration, or use the configured `port_range_start` port. Set the admin
password on first access.

**Trusted networks only.** The devnet publishes Portainer's HTTP port without
a loopback-only bind and mounts the Docker socket. Portainer can manage the
entire Docker daemon, not only the devnet. Restrict access with the host
firewall and do not expose the port publicly or forward it from another network.

`stop` leaves Portainer running but removes the devnet service containers.
Portainer cannot show Docker logs for removed containers. For prior runs, use
`run/<run-id>/setup.log` and any startup snapshots in `run/<run-id>/logs/`.
For logs produced after startup, capture `docker logs` while the container
still exists.

### Container Architecture

| Container | Image | Ports inside the container |
|-----------|-------|----------------------------|
| `foc-<run-id>-lotus` | `foc-lotus` | 1234 (API), 1235 (P2P) |
| `foc-<run-id>-lotus-miner` | `foc-lotus-miner` | 2345 (API) |
| `foc-<run-id>-curio-N` | `foc-curio` | API, UI, and PDP service |
| `foc-<run-id>-postgres-N` | `postgres` | 5432 |
| `foc-<run-id>-scylla-N` | `scylladb/scylla` | 9042 (CQL) |
| `foc-<run-id>-portainer` | `portainer/portainer-ce` | 9000 (HTTP) |

`N` is the one-based provider index. The `foc-builder` image is used by
short-lived deployment commands; there is no persistent builder to `docker exec` into.

### Network Topology

foc-devnet creates user-defined Docker bridge networks with these names:

- `foc_<run-id>_lot-net` — Lotus, with connections from Lotus Miner and Curio.
- `foc_<run-id>_lot-m-net` — Lotus Miner's separate network.
- `foc_<run-id>_cur-m-net-N` — Curio provider `N` and its Postgres and Scylla containers.

These networks separate container traffic but do not restrict host access to
published ports. Use [the exported host RPC URL](#accessing-lotus-api) for
Lotus and the export's `info.lotus_miner.api_port` and `info.pdp_sps`
entries for the miner, PDP services, and databases.

### Port Management

The default host port range is 5700–5799. Portainer uses the first port.
Lotus, Lotus Miner, the Curio instances, and the databases get dynamically
assigned host ports from the rest of the range. Do not assume that a container
port is also its host port. Use `foc-devnet status` or the exported addresses
to locate a running service.

---

## Dependency Repository Management

Sources are selected through [`init`](#init) and recorded in
[`config.toml`](#config-structure).

### Required Dependent Repositories

- **[lotus](https://github.com/filecoin-project/lotus)** - Filecoin daemon
- **[curio](https://github.com/filecoin-project/curio)** - Storage provider (PDP)
- **[filecoin-services](https://github.com/FilOzone/filecoin-services)** - FOC smart contracts
- **[pdp](https://github.com/FilOzone/pdp)** - PDP contracts; optional independent override mounted over `service_contracts/lib/pdp`
- **[multicall3](https://github.com/mds1/multicall3)** - Multicall3 contract

### Dependent Version Strategy

Default versions for these repositories are defined in code (see [`src/config.rs`](src/config.rs) `Config::default()`).

Use a tag or commit to pin a checkout; a branch can move. `latesttag` resolves
to a specific tag during `init` and is saved as `GitTag`, so an ordinary re-init
using that config does not automatically select a newer tag.

**Updating defaults:** See [How Defaults Work](#how-defaults-work) for information on how defaults are defined and the steps to apply updates.

### Using Local Dependency Repositories

**For active development:**

```bash
foc-devnet init \
    --lotus local:/home/user/dev/lotus \
    --curio local:/home/user/dev/curio \
    --filecoin-services local:/home/user/dev/filecoin-services \
    --pdp local:/home/user/dev/pdp
```

**Mixed approach:**

```bash
foc-devnet init \
    --lotus gitbranch:master \
    --curio local:/home/user/dev/curio
```

### Sharing Configuration

Copy a valid config into a clean base directory before `init`. For a shared
setup, use the same foc-devnet version and pin the dependencies. `LocalSource`
paths must exist on the recipient's machine.

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
mkdir -p "$base"
cp -i shared-config.toml "$base/config.toml"
foc-devnet init
```

---

### Reproducible Builds

Pin each dependency to a specific release tag, as in the
[configuration example](#config-structure), or to an exact commit. For a
commit pin, replace the component's `GitTag` table with a `GitCommit` table
(for example `[lotus.GitCommit]`), keep the `url`, and use `commit` instead of
`tag`. Commit pins avoid relying on tags remaining unchanged.

---

## Lifecycle Overview

The usual workflow is `init` → `build lotus` and `build curio` → `start` →
`stop`. A later `start` creates a fresh run; it does not require a rebuild
unless the binaries have changed or were removed. See
[Normal Start Behavior](#normal-start-behavior).

### Detailed Start Sequence

#### Before Steps

Startup checks host name resolution, attempts to stop the existing cluster,
records the new run, and sets up Portainer, run networks, and a fresh genesis.
See [Troubleshooting](#troubleshooting) if startup fails before reaching the
service steps.

#### Steps

Startup launches Lotus and Lotus Miner, funds test accounts, deploys contracts,
and starts the Curio databases and daemons. It registers the providers,
approves and endorses the configured subsets, and configures `USER_1` to use
Warm Storage. The other test users are funded but not configured for Warm
Storage. Sequential startup is the default. The experimental parallel
schedule includes its own prerequisites-check step before service setup.
For exact ordering and parallel groups, see `create_steps` and
`create_step_epochs` in [the startup implementation](src/commands/start/mod.rs).

#### Post Start Steps

Startup finalization attempts to save container log snapshots and a status
snapshot after the service steps finish. It then attempts the
[`devnet-info.json` export](#key-files) if the service steps succeeded.

### Step Implementation Pattern

The [startup step implementation](src/commands/start/step/) defines the
execution interface and shared context.

---

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
on a running devnet:

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
run_id=$(cat "$base/state/current_run_id.txt")
docker exec "foc-$run_id-postgres-1" psql -U curio -d curio -c '\dn'
docker exec "foc-$run_id-scylla-1" cqlsh -e 'DESCRIBE KEYSPACES;'
```

---

## Troubleshooting

### Port conflicts
Check which process is using the conflicting host port, for example
`lsof -i :5700`. If the range needs to change, update `port_range_start` and
`port_range_count` in [the configuration](#configuration-system), then start a
new run. If the current cluster is still running, stop it from its original
base directory first.

### Container won't start
Look in the latest run's `setup.log`, `post_start_status.log`, and `logs/`
for startup diagnostics. For an existing container, get its name from
`docker ps -a` and inspect its current logs with `docker logs`. A container
removed during startup finalization or `stop` will not have live Docker logs.

### Build failures
Check available disk space (`df -h`), Docker usage (`docker system df`), and
the CLI build logs under `logs/build/` in the [base directory](#directory-structure).
If the foc-devnet images need to be rebuilt, use the scoped
[`stop` then `clean --images`](#clean) procedure, followed by `init`,
`build lotus`, and `build curio`. This removes local run data; save anything
you need first. Use `init` to build images with the host UID/GID settings
foc-devnet expects, rather than building the image with bare `docker build`.

### Network issues
For host resolution errors, see [Setup Requirements](#setup-requirements).
To inspect the current run's Lotus bridge:

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
run_id=$(cat "$base/state/current_run_id.txt")
docker network inspect "foc_${run_id}_lot-net"
```

Normal [`stop`](#stop) attempts to remove the devnet networks. If one remains,
inspect that exact network and its attachments before removing it manually.

---

## Additional User Actions

### Custom Genesis Block

For inspection, generated genesis data lives under
`docker/volumes/run-specific/<run-id>/genesis/` in the base directory.
There is no CLI or configuration setting for a custom genesis, and startup
does not pause for template edits. Source changes belong in the
[genesis implementation](src/commands/start/genesis/).

### Monitoring with Portainer
See [Portainer: Your Debugging Companion](#portainer-your-debugging-companion)
for access, log retention, and the trusted-network requirement.

### Accessing Lotus API
Use the exported URL instead of assuming a host port. This example uses `jq`
and `curl` to query the running Lotus node:

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
info="$base/state/latest/devnet-info.json"
run_id=$(jq -er '.info.run_id' "$info")
rpc_url=$(jq -er '.info.lotus.host_rpc_url' "$info")
token=$(cat "$base/docker/volumes/run-specific/$run_id/lotus-data/token")
curl -fsS "$rpc_url" \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $token" \
  -d '{"jsonrpc":"2.0","method":"Filecoin.ChainHead","params":[],"id":1}'
```

Inside the Lotus container the token is at `/home/foc-user/.lotus-local-net/token`.
Do not share the token.

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

## Scenario Tests

Scenario tests run serially against one running devnet. The Python runner uses
the standard library, but scenarios also invoke Docker and Foundry and run
Node.js clients. Use Python 3.11+, Git, Node.js 24+, npm, and pnpm. The
[prerequisite script](scripts/setup-scenarios-prerequisites.sh) checks the
JavaScript tools and installs Foundry; the scenarios install client dependencies
and need network access for downloads.

### Running scenarios

From the repository root, after starting the devnet and confirming that its
export is present:

```bash
./scripts/setup-scenarios-prerequisites.sh
python3 scenarios/run.py

# Or run a single scenario without the suite report
python3 scenarios/test_basic_balances.py
```

The default input is `state/latest/devnet-info.json` under the selected base
directory. To specify a run explicitly, set `DEVNET_INFO` to
`<base-dir>/run/<run-id>/devnet-info.json`; the corresponding services must
still be running. The suite writes a Markdown report to
`~/.foc-devnet/state/latest/scenario_report.md` by default, independently of
`FOC_DEVNET_BASEDIR` and `DEVNET_INFO`. When using a custom base directory or
an explicit run, set `REPORT_FILE` to the desired report path (in an existing
directory). For example:

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
run_id=$(cat "$base/state/current_run_id.txt")
PATH="$HOME/.foc-devnet/artifacts/foundry/bin:$PATH" \
  DEVNET_INFO="$base/run/$run_id/devnet-info.json" \
  REPORT_FILE="$base/run/$run_id/scenario_report.md" \
  python3 scenarios/run.py
```

The prerequisite script installs Foundry under `~/.foc-devnet` regardless of
the selected base directory. Adding that Foundry directory to `PATH`, as in
the example above, makes it available to scenarios using a custom base directory.

### CI integration

CI runs scenarios after startup and uploads the report as a workflow artifact.
Nightly or manually enabled reporting can also create or update a GitHub issue.
See [CI Dependency Profiles](ci/README.md) for dependency selection and pins.
