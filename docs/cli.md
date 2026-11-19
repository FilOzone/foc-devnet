# CLI reference

Commands and options. For a first run from source, see the [quick start](../README.md#-quick-start).
If the executable is not on your `PATH`, use `cargo run --` in place of
`foc-devnet` from the repository root.

## Commands Reference

### `clean`
Removes the contents of the selected [base directory](configuration.md#foc_devnet_basedir),
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

Run `stop`, then check `docker ps` for remaining devnet services (Portainer
can remain running). If any are still running, resolve the teardown issue
before deleting their data. To remove local data and foc-devnet images while
keeping the configuration for the next `init`:

```bash
foc-devnet clean --images
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
`build curio`. Startup first checks [host name resolution](networking.md#setup-requirements),
then attempts to [stop existing devnet services](#stop) on the same Docker daemon.

```bash
foc-devnet start [--parallel]
```

Startup is sequential by default. `--parallel` enables **experimental**
concurrent execution of independent steps; use the default when troubleshooting.
See the [startup sequence](lifecycle.md#lifecycle-overview) and
[run outputs](lifecycle.md#key-files) for what to expect.

### `stop`
Stops and removes devnet service containers and attempts to remove their Docker
networks. It clears the current run ID from the selected base directory, but
preserves run files and leaves [Portainer](networking.md#portainer-your-debugging-companion)
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
