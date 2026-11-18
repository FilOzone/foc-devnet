# Configuration

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
Use a nonempty [port range](networking.md#port-management) with enough available ports for
the services configured.

### How Defaults Work

`init` writes [`Config::default()`](../src/config.rs) when no configuration file
exists. An upgrade does not replace existing settings. To regenerate the
configuration, back it up, run [`foc-devnet stop`](cli.md#stop), and
[check that no devnet services remain](cli.md#clean). Then run:

```bash
foc-devnet clean --all && foc-devnet init
```

This deletes local run data as well as the old configuration. Plain `clean`
preserves `config.toml` for the next `init`.

### Editing Config

Edit `config.toml` to change the ports or provider counts for the next `start`.
For source changes, use [`init` source overrides](cli.md#init) when reinitializing.
Keep ports and counts at the top level, before the component tables.

### Environment Variables

#### `FOC_DEVNET_BASEDIR`

Overrides the default `~/.foc-devnet` base directory. Use it to keep different
configurations and their files on separate paths, or to put the data on another
disk. It does **not** isolate Docker resources. Use only one running foc-devnet
per Docker daemon: [start and stop share cleanup scope](cli.md#stop), startup
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
in these guides assume `FOC_DEVNET_BASEDIR` is unset or already expanded to an absolute
path.

## Dependency Repository Management

Sources are selected through [`init`](cli.md#init) and recorded in
[`config.toml`](#config-structure).

### Required Dependent Repositories

- **[lotus](https://github.com/filecoin-project/lotus)** - Filecoin daemon
- **[curio](https://github.com/filecoin-project/curio)** - Storage provider (PDP)
- **[filecoin-services](https://github.com/FilOzone/filecoin-services)** - FOC smart contracts
- **[pdp](https://github.com/FilOzone/pdp)** - PDP contracts; optional independent override mounted over `service_contracts/lib/pdp`
- **[multicall3](https://github.com/mds1/multicall3)** - Multicall3 contract

### Dependent Version Strategy

Default versions for these repositories are defined in code (see [`src/config.rs`](../src/config.rs) `Config::default()`).

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

### Reproducible Builds

Pin each dependency to a specific release tag, as in the
[configuration example](#config-structure), or to an exact commit. For a
commit pin, replace the component's `GitTag` table with a `GitCommit` table
(for example `[lotus.GitCommit]`), keep the `url`, and use `commit` instead of
`tag`. Commit pins avoid relying on tags remaining unchanged.
