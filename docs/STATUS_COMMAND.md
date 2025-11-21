# Status Command - Port Display Feature

## Overview

The `foc-localnet status` command now displays port information for running containers, showing which ports are exposed and accessible on the host machine.

## Port Display Format

When a container is running, ports are displayed in the format:
```
PORT(SERVICE) PORT(SERVICE) ...
```

### Color Coding

- **Green**: Port is exposed by the container AND accessible on the host machine
- **Red**: Port is expected but either not exposed or not accessible
- **N/A** (grey): Container is not running (no port information available)

## Example Output

```
⚙️ System Status
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
Service       Status   Container        Ports
Lotus Daemon  Stopped  foc-lotus        N/A
Lotus Miner   Stopped  foc-lotus-miner  N/A
Curio         Stopped  foc-curio        N/A
YugabyteDB    Running  foc-yugabyte     5433(YSQL) 9042(YCQL) 7000(M-RPC) 9000(M-UI) 7100(T-RPC) 9100(T-UI) 15433(Web)
Builder       Stopped  foc-builder      N/A
```

## Container Port Mappings

### YugabyteDB (foc-yugabyte)

| Port  | Short Name | Full Name                         | Description                          |
|-------|------------|-----------------------------------|--------------------------------------|
| 5433  | YSQL       | YugabyteDB SQL                    | PostgreSQL-compatible SQL interface  |
| 9042  | YCQL       | YugabyteDB CQL                    | Cassandra-compatible interface       |
| 7000  | M-RPC      | Master RPC                        | Master server RPC                    |
| 9000  | M-UI       | Master UI                         | Master admin web interface           |
| 7100  | T-RPC      | TServer RPC                       | Tablet server RPC                    |
| 9100  | T-UI       | TServer UI                        | TServer admin web interface          |
| 15433 | Web        | Web UI                            | Main YugabyteDB dashboard            |

### Lotus Daemon (foc-lotus)

| Port | Short Name | Description           |
|------|------------|-----------------------|
| 1234 | API        | Lotus API endpoint    |
| 5678 | P2P        | P2P networking port   |

### Lotus Miner (foc-lotus-miner)

| Port | Short Name | Description              |
|------|------------|--------------------------|
| 2345 | API        | Lotus Miner API endpoint |

### Curio (foc-curio)

| Port  | Short Name | Description        |
|-------|------------|--------------------|
| 12300 | API        | Curio API endpoint |

### Builder (foc-builder)

No ports configured (build-only container).

## How It Works

1. **Container Detection**: The command checks which containers are running using `docker ps`
2. **Port Mapping Retrieval**: For running containers, it retrieves port mappings using `docker port <container>`
3. **Accessibility Check**: Each expected port is tested using TCP connection with a 100ms timeout
4. **Status Display**: Ports are colored green (accessible) or red (not accessible)

## Implementation Details

### Port Accessibility Test

The system performs a TCP connection test to verify port accessibility:
- Connects to `127.0.0.1:<port>`
- Timeout: 100ms
- Green: Connection successful
- Red: Connection failed or port not exposed

### Adding New Container Port Mappings

To add port mappings for a new container, edit the `get_expected_ports` function in `src/commands/status.rs`:

```rust
fn get_expected_ports(container_name: &str) -> Vec<(u16, &'static str)> {
    match container_name {
        "foc-yugabyte" => vec![
            (5433, "YSQL"),
            (9042, "YCQL"),
            // ... more ports
        ],
        "foc-your-container" => vec![
            (8080, "HTTP"),
            (8443, "HTTPS"),
        ],
        _ => vec![],
    }
}
```

## Troubleshooting

### All Ports Showing Red

If all ports are showing red despite the container running:
1. Check if ports are properly exposed in the Docker run command
2. Verify no firewall is blocking the ports
3. Check if another process is using the ports
4. Use `docker port <container_name>` to verify port mappings

### Port Shows Green But Service Not Accessible

The green status only indicates the port is listening. The actual service might:
- Still be initializing
- Have authentication/authorization issues
- Be bound to a different interface

### Mixed Green/Red Ports

This is normal during service startup:
- Some services start faster than others
- Run the status command again after a few seconds
- Check container logs for any startup errors

## Performance Considerations

- Port accessibility checks use 100ms timeout
- Multiple ports are checked sequentially
- For containers with many ports, status command may take a few hundred milliseconds longer
- No performance impact when containers are stopped (no port checks performed)

## Future Enhancements

Potential improvements for the port display feature:
- Parallel port checking for faster results
- Display HTTP/HTTPS URLs for web interfaces
- Health check integration (beyond simple port listening)
- Port response time display
- Historical uptime tracking per port
