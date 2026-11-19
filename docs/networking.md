# Networking and troubleshooting

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

For GitHub Actions, see the [hostname setup step in CI](../.github/workflows/ci_run.yml#L306-L308).

### Validation

`foc-devnet start` checks for a loopback address for `host.docker.internal`
before stopping any existing cluster. If it fails, correct the host mapping
before retrying.

## Docker and Networking

Docker runs the devnet services and the Lotus and Curio builds. See the
[installation prerequisites](../README.md#prerequisites) for the host requirements.

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
| `foc-<run-id>-lotus` | `foc-lotus` | 1234 (API), 1946 (P2P) |
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
published ports. Use [the exported host RPC URL](integrations.md#accessing-lotus-api) for
Lotus and the export's `info.lotus_miner.api_port` and `info.pdp_sps`
entries for the miner, PDP services, and databases.

### Port Management

The default host port range is 5700–5799. Portainer uses the first port.
Lotus, Lotus Miner, the Curio instances, and the databases get dynamically
assigned host ports from the rest of the range. Do not assume that a container
port is also its host port. Use `foc-devnet status` or the exported addresses
to locate a running service.

## Troubleshooting

### Port conflicts
Check which process is using the conflicting host port, for example
`lsof -i :5700`. If the range needs to change, update `port_range_start` and
`port_range_count` in [the configuration](configuration.md#configuration-system), then start a
new run. If the current cluster is still running, stop it from its original
base directory first.

### Container won't start
Start with the failed command's output or `setup.log`. Early failures can
happen before `state/latest` is updated or before `post_start_status.log`
and container log snapshots are written. For an existing container, find
its name with `docker ps -a` and inspect it with `docker logs`. Docker
cannot show live logs for a removed container.

### Build failures
Check available disk space (`df -h`), Docker usage (`docker system df`), and
the CLI build logs under `logs/build/` in the [base directory](lifecycle.md#directory-structure).
If the foc-devnet images need to be rebuilt, use the scoped
[`stop` then `clean --images`](cli.md#clean) procedure, followed by `init`,
`build lotus`, and `build curio`. This removes local run data; save anything
you need first. Use `init` to build images with the host UID/GID settings
foc-devnet expects, rather than building the image with bare `docker build`.

### Network issues
For host resolution errors, see [Setup Requirements](#setup-requirements).
To inspect the current run's Lotus bridge, use `jq` and Docker:

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
run_id=$(jq -er '.run_id' "$base/state/current_runid.json") &&
docker network inspect "foc_${run_id}_lot-net"
```

Normal [`stop`](cli.md#stop) attempts to remove the devnet networks. If one remains,
inspect that exact network and its attachments before removing it manually.
