# Fast Init Guide for Development

This guide shows how to use local code repositories and pre-cached artifacts to speed up the `init` process during development.

## Quick Start

For fast iteration during development, use these steps to avoid re-downloading large artifacts:

### 1. Pre-cache Artifacts (One-time Setup)

Before running init, copy pre-downloaded artifacts to the foc-localnet directories:

```bash
# Create artifacts directory if it doesn't exist
mkdir -p ~/.foc-localnet/artifacts

# Copy yugabyte tarball from stash
cp ~/stash/yugabyte/yugabyte-2.25.1.0-b381-linux-x86_64.tar.gz ~/.foc-localnet/artifacts/

# The init process will extract this automatically
```

### 2. Run Init with Local Repositories

Use local code paths instead of cloning from GitHub:

```bash
cargo run init \
  --curio local:/home/redpanda/code/curio \
  --filecoin-services local:/home/redpanda/code/filecoin-services \
  --lotus local:/home/redpanda/code/lotus
```

This tells foc-localnet to:
- Create symlinks to your local code directories instead of cloning
- Skip git operations
- Use the code exactly as it exists on your filesystem

### 3. Cache Proof Parameters (After Init Completes)

After the first init completes, cache the proof parameters for subsequent inits:

```bash
# Create proof parameters directory
mkdir -p ~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters

# Copy from stash (this is ~2GB, so it takes a moment)
cp -r ~/stash/filecoin-proof-parameters/* ~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters/
```

**Note**: This step must be done AFTER init because the directory structure `~/.foc-localnet/artifacts/docker/volumes/` is created during init.

## Complete Fast Workflow

```bash
# One-time setup (do this once)
mkdir -p ~/.foc-localnet/artifacts
cp ~/stash/yugabyte/yugabyte-2.25.1.0-b381-linux-x86_64.tar.gz ~/.foc-localnet/artifacts/

# Run init with local paths
cargo run init \
  --curio local:/home/redpanda/code/curio \
  --filecoin-services local:/home/redpanda/code/filecoin-services \
  --lotus local:/home/redpanda/code/lotus

# After first init completes, cache proof parameters for next time
mkdir -p ~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters
cp -r ~/stash/filecoin-proof-parameters/* ~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters/

# Now you can quickly clean and re-init for testing:
cargo run clean
# Re-cache yugabyte
cp ~/stash/yugabyte/yugabyte-2.25.1.0-b381-linux-x86_64.tar.gz ~/.foc-localnet/artifacts/
# Re-run init (much faster now)
cargo run init \
  --curio local:/home/redpanda/code/curio \
  --filecoin-services local:/home/redpanda/code/filecoin-services \
  --lotus local:/home/redpanda/code/lotus
# Re-cache proof parameters
cp -r ~/stash/filecoin-proof-parameters/* ~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters/
```

## What Gets Cached

| Artifact | Size | Location | Notes |
|----------|------|----------|-------|
| Yugabyte | ~422 MB | `~/.foc-localnet/artifacts/yugabyte-*.tar.gz` | Extracted automatically during init |
| Proof Parameters | ~2 GB | `~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters/` | Used by lotus during proof generation |
| Local Code | 0 bytes | Symlinks to `~/code/{lotus,curio,filecoin-services}` | No copying, just symlinks |

## Benefits

- **No git clones**: Instant repository "download" using symlinks
- **No large downloads**: Yugabyte (~422 MB) and proof params (~2 GB) copied from local cache
- **Quick iteration**: `clean` + `init` cycle is much faster
- **Code editing**: Changes in `~/code/*` are immediately reflected in foc-localnet

## Reverting to Git-based Init

To go back to cloning from GitHub, simply omit the local paths:

```bash
cargo run init
```

This will use the default configuration which clones from:
- Lotus: `https://github.com/filecoin-project/lotus.git` (tag: v1.34.0)
- Curio: `https://github.com/filecoin-project/curio.git` (branch: pdpv0)
- Filecoin Services: `https://github.com/FilOzone/filecoin-services.git` (tag: v1.0.0)

## Troubleshooting

### "Repository already exists" error

If you see this error when switching between local and git-based init:

```bash
cargo run clean  # This removes all symlinks and cloned repositories
cargo run init   # Try again
```

### Yugabyte extraction fails

If extraction fails, make sure you have the correct tarball:

```bash
ls -lh ~/.foc-localnet/artifacts/yugabyte-*.tar.gz
# Should show: yugabyte-2.25.1.0-b381-linux-x86_64.tar.gz (~422M)
```

### Proof parameters not found during build

The proof parameters are only needed during `build` and `start`, not during `init`. If you see errors:

```bash
# Re-copy the proof parameters
rm -rf ~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters
mkdir -p ~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters
cp -r ~/stash/filecoin-proof-parameters/* ~/.foc-localnet/artifacts/docker/volumes/filecoin-proof-parameters/
```
