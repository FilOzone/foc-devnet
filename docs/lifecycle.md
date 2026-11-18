# Run lifecycle and cleanup

## Lifecycle Overview

The usual workflow is `init` → `build lotus` and `build curio` → `start` →
`stop`. A later `start` creates a fresh run; it does not require a rebuild
unless the binaries have changed or were removed. See
[Normal Start Behavior](#normal-start-behavior).

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

Use [`clean`](cli.md#clean) after stopping if none of the local runs need to be kept.
To keep the recorded chain but remove other run data, use Bash and `jq`.
Capture the ID before `stop` clears it. The preview also includes logs from
other CLI commands, including the `stop` command. Do not start another devnet
during cleanup.

```bash
(
  set -e
  base=$(cd -- "${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}" && pwd -P)
  export FOC_DEVNET_BASEDIR="$base"
  keep=$(jq -er '.run_id' "$base/state/current_runid.json")
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
  running=$(docker ps --filter 'name=foc-' --format '{{.Names}}')
  while IFS= read -r name; do
    case "$name" in
      foc-portainer|foc-*-portainer) ;;
      foc-*)
        printf 'Container still running: %s; no data removed.\n' "$name" >&2
        exit 1 ;;
    esac
  done <<< "$running"

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
│   ├── current_runid.json           # Cleared by stop
│   └── latest                       # Symlink to latest recorded run directory
└── tmp/
```

### Key Files

Use `devnet-info.json` for scripts and SDKs. The export includes a top-level
`version` and an `info` object with the run ID, host RPC URL, contract addresses,
users and their **test private keys**, and provider details. See the
[schema](../examples/devnet-schema.js) and [validation instructions](../examples/README.md).
Do not publish the file or use its keys outside the devnet.
