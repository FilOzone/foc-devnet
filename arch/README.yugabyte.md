# YugabyteDB Docker Container Setup

## Overview

The `foc-yugabyte` Docker container provides a YugabyteDB instance for the foc-localnet environment. YugabyteDB is a distributed SQL database that is PostgreSQL-compatible and used as the backend database for the Filecoin network components.

## Building the Image

The YugabyteDB Docker image is automatically built during the `foc-localnet init` process as part of the `build_and_cache_docker_images` step.

### Build Process

1. The Dockerfile (`docker/yugabyte/Dockerfile`) uses Ubuntu 24.04 as the base image
2. Installs required dependencies (wget, ca-certificates, locales, python3)
3. Configures locales to prevent startup errors (`en_US.UTF-8`)
4. Copies the YugabyteDB binary from the `~/.foc-localnet/artifacts/yugabyte` directory
5. Runs the post-install script to complete the installation
6. The image is saved as `foc-yugabyte.tar` in `~/.foc-localnet/artifacts/docker/images/`

## Exposed Ports

YugabyteDB requires several ports for different services:

| Port  | Service                           | Description                                      |
|-------|-----------------------------------|--------------------------------------------------|
| 5433  | YSQL API                          | PostgreSQL-compatible SQL interface              |
| 9042  | YCQL API                          | Cassandra-compatible query language interface    |
| 7000  | YB-Master RPC                     | Master server RPC communication                  |
| 9000  | YB-Master Admin UI                | Master server web admin interface                |
| 7100  | YB-TServer RPC                    | Tablet server RPC communication                  |
| 9100  | YB-TServer Admin UI               | Tablet server web admin interface                |
| 15433 | YugabyteDB Web UI                 | Main web dashboard for cluster management        |

### Port Usage Details

- **5433 (YSQL)**: Primary interface for PostgreSQL-compatible queries. Most applications will connect to this port.
- **9042 (YCQL)**: Cassandra-compatible API, useful for applications using CQL.
- **15433 (Web UI)**: Web-based dashboard showing cluster health, performance metrics, and node status. Access via http://localhost:15433
- **7000, 7100**: Internal RPC ports for master and tablet server communication
- **9000, 9100**: Administrative web interfaces for master and tablet servers

## Starting the Container

The container is configured to start YugabyteDB automatically with the following settings:

```bash
/yugabyte/bin/yugabyted start \
  --advertise_address=0.0.0.0 \
  --master_flags=rpc_bind_addresses=0.0.0.0 \
  --tserver_flags=rpc_bind_addresses=0.0.0.0 \
  --daemon=false
```

### Configuration Details

- **advertise_address=0.0.0.0**: Makes the service accessible from outside the container
- **rpc_bind_addresses=0.0.0.0**: Allows RPC connections from any network interface
- **daemon=false**: Runs in foreground mode (suitable for Docker containers)

## Running the Container

To run the YugabyteDB container manually:

```bash
docker run -d \
  --name foc-yugabyte \
  -p 5433:5433 \
  -p 9042:9042 \
  -p 7000:7000 \
  -p 9000:9000 \
  -p 7100:7100 \
  -p 9100:9100 \
  -p 15433:15433 \
  foc-foc-yugabyte
```

### With Volume Persistence

For data persistence across container restarts:

```bash
docker run -d \
  --name foc-yugabyte \
  -p 5433:5433 \
  -p 9042:9042 \
  -p 7000:7000 \
  -p 9000:9000 \
  -p 7100:7100 \
  -p 9100:9100 \
  -p 15433:15433 \
  -v yugabyte-data:/yugabyte/data \
  foc-foc-yugabyte
```

## Verifying Installation

### Web UI

1. Access the YugabyteDB Web UI at http://localhost:15433
2. You should see the dashboard with cluster status and node health

### CLI Status Check

Execute inside the running container:

```bash
docker exec -it foc-yugabyte /yugabyte/bin/yugabyted status
```

### Connect via YSQL

Connect using any PostgreSQL client:

```bash
docker exec -it foc-yugabyte /yugabyte/bin/ysqlsh -h 0.0.0.0 -p 5433
```

Or from your host machine (requires PostgreSQL client):

```bash
psql -h localhost -p 5433 -U yugabyte
```

## Troubleshooting

### Locale Errors

The Dockerfile pre-configures locales, but if you encounter locale-related errors:

```bash
docker exec -it foc-yugabyte locale-gen en_US.UTF-8
docker restart foc-yugabyte
```

### Port Conflicts

If ports are already in use, modify the port mappings when running the container:

```bash
docker run -d \
  --name foc-yugabyte \
  -p 15434:5433 \  # Changed host port to 15434
  # ... other port mappings
  foc-foc-yugabyte
```

### Container Logs

View container logs for debugging:

```bash
docker logs foc-yugabyte
docker logs -f foc-yugabyte  # Follow logs in real-time
```

## Integration with foc-localnet

The YugabyteDB container integrates seamlessly with foc-localnet:

1. Image is built during `foc-localnet init`
2. Image is cached in `~/.foc-localnet/artifacts/docker/images/foc-yugabyte.tar`
3. Can be started/stopped via foc-localnet commands (future implementation)
4. Provides the database backend for Filecoin network components

## References

- [YugabyteDB Official Documentation](https://docs.yugabyte.com/)
- [YugabyteDB Quick Start Guide](https://docs.yugabyte.com/preview/quick-start/)
- [YSQL API Reference](https://docs.yugabyte.com/preview/api/ysql/)
