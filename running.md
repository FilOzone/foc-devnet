# Lotus and Lotus-Miner Tipset Generation - RESOLVED ✅

## Summary
Successfully debugged and fixed the lotus and lotus-miner setup. The network is now properly generating tipsets (producing blocks).

## Issues Found and Fixed

### 1. Prefunded Accounts Configuration
**Issue**: The setup was creating 2 additional prefunded accounts beyond what the minimal Lotus setup requires.
**Fix**: Set `NUM_PREFUNDED_KEYS = 0` in `src/commands/start/genesis/constants.rs` to match the official Lotus local network setup.

### 2. Lotus API Initialization Timing
**Issue**: The lotus daemon takes 1-2 minutes to fully initialize, but the code was only waiting 10 seconds before considering it ready.
**Fix**: Added proper wait logic in `src/commands/start/lotus.rs`:
- Wait for the `api` file to be created in the lotus data directory
- Increased timeout to 180 seconds
- Added verification that the API is actually responding

### 3. Lotus-Miner Startup Race Condition
**Issue**: The lotus-miner container was starting before the lotus daemon API was fully ready, causing connection failures.
**Fix**: Added retry loop in `src/commands/start/lotus_miner.rs`:
```bash
until /usr/local/bin/lotus-bins/lotus version >/dev/null 2>&1; do \
  echo "Lotus API not ready yet, waiting..." && sleep 2; \
done
```

### 4. Docker Networking Issue (Critical)
**Issue**: The lotus container was using bridge networking (with port mappings) while lotus-miner was using `--network host`. This caused them to be on different networks and unable to communicate.
**Fix**: Changed lotus-miner to use `--network container:foc-lotus` to share the lotus container's network namespace. This allows both containers to communicate via localhost.

## Testing

Start and stop the network via:
- `cargo run -- start --reset`
- `cargo run -- stop`

Verify tipset generation:
```bash
# Check chain list
docker exec foc-lotus /usr/local/bin/lotus-bins/lotus chain list

# Watch lotus-miner logs
docker logs -f foc-lotus-miner

# Check running containers
docker ps
```

## Results
✅ Lotus daemon starts and initializes properly
✅ Lotus-miner connects successfully
✅ Tipsets are being generated every ~4 seconds
✅ Added automated tipset generation test in lotus-miner post-execution check

The local Filecoin network is now fully functional!