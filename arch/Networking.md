# FOC LocalNet - Networking Architecture

This document describes the networking architecture and port configurations for the FOC (Filecoin-onchain-cloud) LocalNet setup, which involves multiple Docker containers working together to run a local Filecoin blockchain network.

## Overview

The FOC LocalNet consists of four primary components running in separate Docker containers:
- **lotus**: Filecoin execution node (2K network with FEVM support)
- **lotus-miner**: Genesis miner/storage provider node
- **curio**: Separate storage provider (Proof of Data Replication)
- **yugabyte**: Database backend for Curio dealmaking

## Container Network Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                         Docker Network                            │
│                                                                   │
│                        ┌─────────────┐                            │
│                        │   lotus     │                            │
│                        │             │                            │
│                        │  Port 1234  │                            │
│                        │  (API/RPC)  │                            │
│                        │             │                            │
│                        │  Port 1235  │                            │
│                        │  (EthRPC)   │                            │
│                        │             │                            │
│                        │  libp2p     │                            │
│                        │  (dynamic)  │                            │
│                        └──────▲──────┘                            │
│                               │                                   │
│                 ┌─────────────┼─────────────┐                     │
│                 │             │             │                     │
│          ┌──────▼──────┐      │      ┌──────▼──────┐              │
│          │ lotus-miner │      │      │   curio     │              │
│          │             │      │      │             │              │
│          │  Port 2345  │      │      │  Port 12300 │              │
│          │ (Miner API) │      │      │  (GUI)      │              │
│          │             │      │      │             │              │
│          │  libp2p     │      │      │  Port 32100 │              │
│          │  (dynamic)  │      │      │  (API)      │              │
│          │             │      │      │             │              │
│          └─────────────┘      │      │  libp2p     │              │
│                               │      │  (dynamic)  │              │
│           (Proof of Rep)      │      └──────┬──────┘              │
│                               │             │                     │
│                               │      (Proof of Data Rep)          │
│                               │             │                     │
│                               │      ┌──────▼──────┐              │
│                               │      │  yugabyte   │              │
│                               │      │             │              │
│                               │      │  5433 (SQL) │              │
│                               │      │  9000 (UI)  │              │
│                               │      │  9100 (UI)  │              │
│                               │      └─────────────┘              │
│                               │                                   │
│                     (Both connect to lotus chain,                 │
│                      operate as independent SPs)                  │
└──────────────────────────────────────────────────────────────────┘
```

## Component Port Details

### 1. Lotus (Filecoin Daemon)

The Lotus daemon is the core blockchain node that maintains the Filecoin state and exposes APIs for interaction.

#### Primary Ports

| Port | Protocol | Purpose | External Access | Notes |
|------|----------|---------|-----------------|-------|
| 1234 | HTTP/JSON-RPC | Lotus API endpoint | Required | Main API for Lotus operations, wallet management, chain queries |
| 1235 | HTTP/JSON-RPC | Ethereum RPC (FEVM) | Required | eth_* JSON-RPC methods for FEVM compatibility, disabled by default |
| Dynamic | TCP | libp2p networking | Optional | P2P communication port (auto-allocated, typically ~40395+) |

#### Configuration Details

**API Endpoint (`1234`)**:
- Default location: `http://127.0.0.1:1234/rpc/v1`
- Used for:
  - Chain synchronization queries
  - Wallet operations (create, import, export)
  - Message pool operations
  - Actor state queries
  - Block and tipset retrieval
- Authentication: Token-based (stored in `~/.lotus/token`)

**Ethereum RPC Endpoint (`1235`)**:
- Default location: `http://127.0.0.1:1234/rpc/v1` (same as API, different methods)
- Must be explicitly enabled via config:
  ```toml
  [Fevm]
    EnableEthRPC = true
  
  [ChainIndexer]
    EnableIndexer = true  # Required for EthRPC
  ```
- Provides Ethereum-compatible JSON-RPC methods:
  - `eth_blockNumber`, `eth_getBlockByHash`, `eth_getBlockByNumber`
  - `eth_call`, `eth_sendTransaction`, `eth_getTransactionReceipt`
  - `eth_getLogs`, `eth_getBalance`, `eth_getCode`
  - Compatible with web3.js, ethers.js, and other Ethereum client libraries
- Essential for interacting with FEVM smart contracts

**libp2p Networking**:
- Configurable via environment variables or config.toml:
  ```toml
  [Libp2p]
    ListenAddresses = ["/ip4/0.0.0.0/tcp/0"]  # 0 = auto-allocate
    # Can specify explicit port: ["/ip4/0.0.0.0/tcp/40395"]
  ```
- Used for:
  - Block propagation
  - Message gossiping
  - Peer discovery and syncing
  - Chain data exchange
- In local devnet: typically set `--bootstrap=false` to prevent external connections

#### Security Considerations
- API token should be protected and not exposed publicly
- In production, bind API to localhost only unless behind proper authentication
- EthRPC should have rate limiting in production environments

---

### 2. Lotus-Miner (Genesis Miner)

The lotus-miner acts as a storage provider and, in local devnet, helps create the genesis block and participates in block production.

#### Primary Ports

| Port | Protocol | Purpose | External Access | Notes |
|------|----------|---------|-----------------|-------|
| 2345 | HTTP/JSON-RPC | Lotus Miner API | Internal | Miner control API, storage management, sealing operations |
| Dynamic | TCP | libp2p networking | Optional | P2P communication for storage deals and data transfer |

#### Configuration Details

**Miner API Endpoint (`2345`)**:
- Default location: `http://127.0.0.1:2345/rpc/v0`
- Used for:
  - Sealing pipeline management
  - Storage deal operations
  - Sector management (pre-seal, seal, prove)
  - Worker management
  - Proving deadlines and WindowPoSt
- Must connect to Lotus daemon on port `1234`
- Authentication: Token-based (stored in `~/.lotusminer/token`)

**libp2p Networking**:
- Similar to Lotus daemon but for storage deals
- Used for:
  - Storage deal proposals
  - Data transfer protocols
  - Retrieval deal handling
- Configuration in `~/.lotusminer/config.toml`:
  ```toml
  [Libp2p]
    ListenAddresses = ["/ip4/0.0.0.0/tcp/0"]
  ```

#### Role in 2K Devnet
- Participates in Proof-of-Replication (standard Filecoin consensus)
- Creates "tipsets" with Lotus daemon to drive blockchain forward
- Uses 2KiB sectors (via `--sector-size 2KiB`) for fast local testing
- Genesis miner (t01000) pre-seals sectors for immediate block production

#### Connection to Lotus
- Requires `FULLNODE_API_INFO` environment variable or config:
  ```bash
  export FULLNODE_API_INFO=<token>:/ip4/127.0.0.1/tcp/1234/http
  ```

---

### 3. Curio (Separate Storage Provider)

Curio is the next-generation storage provider software that registers as a separate SP on the network, supporting Proof of Data Replication.

#### Primary Ports

| Port | Protocol | Purpose | External Access | Notes |
|------|----------|---------|-----------------|-------|
| 12300 | HTTP | Curio GUI Dashboard | Optional | Web-based monitoring and management interface |
| 32100 | HTTP/JSON-RPC | Curio API endpoint | Internal | Curio control API |
| Dynamic | TCP | libp2p networking | Optional | P2P for storage deals and data transfer |

#### Configuration Details

**GUI Dashboard (`12300`)**:
- Web-based interface for monitoring Curio operations
- Real-time visibility into:
  - Task execution (sealing pipeline, PoRep, PoDR)
  - Resource utilization
  - Sector status
  - Deal management
  - Cluster health
- Accessible at: `http://127.0.0.1:12300`

**Curio API Endpoint (`32100`)**:
- RESTful/JSON-RPC API for Curio operations
- Used for:
  - Task management
  - Storage configuration
  - Multi-miner ID operations
  - Deal automation
- May vary based on configuration

**Database Connection**:
- Connects to YugabyteDB on port `5433`
- Requires PostgreSQL-compatible connection string:
  ```
  postgres://yugabyte:yugabyte@yugabyte:5433/yugabyte
  ```
- Used for:
  - Persistent task state
  - Deal metadata
  - Sector tracking
  - Cluster coordination

**Connection to Lotus**:
- Like lotus-miner, requires connection to Lotus daemon:
  ```bash
  export FULLNODE_API_INFO=<token>:/ip4/127.0.0.1/tcp/1234/http
  ```
- Registers as a separate storage provider (new miner ID)
- Does NOT participate in Proof-of-Replication (leaves that to lotus-miner)
- Focuses exclusively on Proof of Data Replication (PoDR)

#### Curio Architecture Benefits
- High availability / zero-downtime capable
- Greedy task management for better scaling
- Multi-miner-ID support on same hardware
- Distributed worker architecture
- No single point of failure

---

### 4. YugabyteDB (Database for Curio)

YugabyteDB provides the PostgreSQL-compatible database backend that powers Curio's dealmaking and cluster coordination.

#### Primary Ports

| Port | Protocol | Purpose | External Access | Notes |
|------|----------|---------|-----------------|-------|
| 5433 | TCP/PostgreSQL | YSQL (SQL API) | Internal | PostgreSQL-compatible wire protocol for Curio |
| 9000 | HTTP | YB Master UI | Optional | Web UI for cluster management and monitoring |
| 7000 | TCP | YB Master RPC | Internal | Master server inter-node communication |
| 9100 | HTTP | YB TServer UI | Optional | Web UI for tablet server monitoring |
| 9042 | TCP | YCQL (optional) | Not used | Cassandra-compatible API (not used by Curio) |
| 12000 | TCP | YCQL RPC | Not used | YCQL protocol endpoint (not used by Curio) |

#### Configuration Details

**YSQL API (`5433`)**:
- PostgreSQL-compatible wire protocol
- Used by Curio for:
  - Persistent storage of task state
  - Deal metadata and history
  - Sector lifecycle tracking
  - Cluster configuration
  - Worker coordination
- Connection parameters:
  - Username: `yugabyte` (default)
  - Password: `yugabyte` (default, should be changed)
  - Database: `yugabyte` (default)
  - Connection string: `postgres://yugabyte:yugabyte@localhost:5433/yugabyte`

**YB Master UI (`9000`)**:
- Web-based administration interface
- Accessible at: `http://127.0.0.1:9000`
- Features:
  - Cluster topology view
  - Table and tablet management
  - Replication and load balancing status
  - System metrics and health
- Useful for debugging and monitoring database state

**YB TServer UI (`9100`)**:
- Tablet server monitoring interface
- Accessible at: `http://127.0.0.1:9100`
- Shows:
  - Tablet operations
  - RPC metrics
  - Storage utilization
  - Performance statistics

**Internal Communication Ports**:
- Port `7000`: Master-to-Master communication (Raft consensus)
- Port `12000`: TServer RPC (not used in single-node setup)
- These ports enable YugabyteDB's distributed architecture features:
  - Multi-region deployments
  - Automatic failover
  - Load balancing
  - Geo-distribution

#### Why YugabyteDB for Curio?
- **PostgreSQL Compatibility**: Curio uses standard PostgreSQL drivers
- **Distributed SQL**: Scales horizontally for larger operations
- **High Availability**: Built-in replication and failover
- **ACID Transactions**: Critical for deal consistency
- **Multi-region Support**: Future-proofs for geo-distributed deployments

---

## Inter-Container Communication

### Lotus ↔ Lotus-Miner
- **Direction**: Bidirectional
- **Protocol**: HTTP/JSON-RPC + libp2p
- **Ports**: 1234 (Lotus API), 2345 (Miner API), libp2p dynamic
- **Purpose**: 
  - Miner queries chain state from Lotus
  - Miner submits blocks to Lotus
  - Chain synchronization
  - Message pool coordination

### Lotus ↔ Curio
- **Direction**: Primarily Curio → Lotus
- **Protocol**: HTTP/JSON-RPC
- **Port**: 1234 (Lotus API)
- **Purpose**:
  - Curio queries chain state
  - Curio submits storage proofs
  - Sector lifecycle management
  - Deal verification

### Curio ↔ YugabyteDB
- **Direction**: Bidirectional
- **Protocol**: PostgreSQL wire protocol
- **Port**: 5433 (YSQL)
- **Purpose**:
  - Persistent task storage
  - Deal metadata management
  - Cluster state coordination
  - Worker task distribution

### Lotus-Miner ↔ Curio
- **Direction**: None (independent SPs)
- **Note**: Both register as separate storage providers on the same Lotus chain
- **Distinction**: 
  - lotus-miner handles Proof-of-Replication
  - Curio handles Proof of Data Replication

---

## Docker Network Configuration

### Recommended Setup: Isolated Network

For a completely isolated local devnet with **no internet access**, use an **internal bridge network**:

```yaml
# docker-compose.yml - Isolated Network Configuration
version: "3.8"

networks:
  filecoin-local:
    driver: bridge
    internal: true  # KEY: Prevents external connectivity
    ipam:
      config:
        - subnet: 172.28.0.0/16
          gateway: 172.28.0.1

services:
  lotus:
    image: <lotus-image>
    container_name: foc-lotus
    hostname: lotus
    networks:
      filecoin-local:
        ipv4_address: 172.28.0.2
    ports:
      - "127.0.0.1:1234:1234"  # API - bind to localhost only
      - "127.0.0.1:1235:1235"  # EthRPC - bind to localhost only
    volumes:
      - ./data/lotus:/home/foc-user/.lotus
    environment:
      - LOTUS_PATH=/home/foc-user/.lotus
      - LOTUS_SKIP_GENESIS_CHECK=_yes_
    # No external DNS or internet access
    dns:
      - 172.28.0.1  # Use Docker's internal DNS only
    
  lotus-miner:
    image: <lotus-image>
    container_name: foc-lotus-miner
    hostname: lotus-miner
    networks:
      filecoin-local:
        ipv4_address: 172.28.0.3
    ports:
      - "127.0.0.1:2345:2345"  # Miner API - localhost only
    volumes:
      - ./data/lotus-miner:/home/foc-user/.lotusminer
      - ./data/genesis-sectors:/home/foc-user/.genesis-sectors
    environment:
      - LOTUS_MINER_PATH=/home/foc-user/.lotusminer
      - FULLNODE_API_INFO=<token>:/ip4/172.28.0.2/tcp/1234/http
    dns:
      - 172.28.0.1
    depends_on:
      - lotus
    
  curio:
    image: <curio-image>
    container_name: foc-curio
    hostname: curio
    networks:
      filecoin-local:
        ipv4_address: 172.28.0.4
    ports:
      - "127.0.0.1:12300:12300"  # GUI - localhost only
      - "127.0.0.1:32100:32100"  # API - localhost only
    volumes:
      - ./data/curio:/home/foc-user/.curio
    environment:
      - CURIO_DB_HOST=172.28.0.5
      - CURIO_DB_PORT=5433
      - CURIO_DB_NAME=yugabyte
      - CURIO_DB_USER=yugabyte
      - CURIO_DB_PASS=yugabyte
      - FULLNODE_API_INFO=<token>:/ip4/172.28.0.2/tcp/1234/http
    dns:
      - 172.28.0.1
    depends_on:
      - lotus
      - yugabyte
    
  yugabyte:
    image: yugabytedb/yugabyte:latest
    container_name: foc-yugabyte
    hostname: yugabyte
    networks:
      filecoin-local:
        ipv4_address: 172.28.0.5
    ports:
      - "127.0.0.1:5433:5433"  # YSQL - localhost only
      - "127.0.0.1:9000:9000"  # Master UI - localhost only
      - "127.0.0.1:9100:9100"  # TServer UI - localhost only
    volumes:
      - ./data/yugabyte:/home/yugabyte/yb_data
    command: ["bin/yugabyted", "start", "--daemon=false"]
    dns:
      - 172.28.0.1
```

### Key Isolation Features

1. **Internal Network**: `internal: true` ensures no external connectivity
   - Containers can communicate with each other
   - Cannot reach the internet
   - Internet cannot reach containers (except via port mappings)

2. **Localhost-only Port Bindings**: `127.0.0.1:HOST_PORT:CONTAINER_PORT`
   - Services accessible only from the host machine
   - Not accessible from other machines on the network
   - Prevents accidental exposure

3. **Internal DNS Only**: Uses Docker's internal DNS resolver
   - No external DNS queries
   - Containers resolve each other by hostname

4. **No Default Gateway to Internet**: Network has gateway but `internal: true` blocks routing

### Alternative: Even More Restrictive

If you want to prevent even localhost access from host machine:

```yaml
networks:
  filecoin-local:
    driver: bridge
    internal: true
    ipam:
      config:
        - subnet: 172.28.0.0/16

services:
  lotus:
    # ... other config ...
    # Remove all 'ports:' sections - no host exposure at all
    # Access only via docker exec
```

Access services using:
```bash
# Execute commands inside containers
docker exec -it foc-lotus lotus wallet list

# Or enter container shell
docker exec -it foc-lotus bash
```

### Environment Variables for Inter-Service Communication

```bash
# Lotus environment
export LOTUS_PATH=~/.lotus-local-net
export LOTUS_SKIP_GENESIS_CHECK=_yes_

# Lotus-Miner environment
export LOTUS_MINER_PATH=~/.lotus-miner-local-net
export FULLNODE_API_INFO=<lotus-token>:/ip4/172.28.0.2/tcp/1234/http

# Curio environment
export CURIO_DB_HOST=172.28.0.5
export CURIO_DB_PORT=5433
export CURIO_DB_NAME=yugabyte
export CURIO_DB_USER=yugabyte
export CURIO_DB_PASS=yugabyte
export FULLNODE_API_INFO=<lotus-token>:/ip4/172.28.0.2/tcp/1234/http
```

---

## Port Summary Table

| Service | Port | Type | Purpose | Expose Externally |
|---------|------|------|---------|------------------|
| lotus | 1234 | TCP/HTTP | Lotus API | Yes (localhost) |
| lotus | 1235 | TCP/HTTP | Ethereum RPC (FEVM) | Yes (if using FEVM) |
| lotus | dynamic | TCP | libp2p P2P | Optional |
| lotus-miner | 2345 | TCP/HTTP | Miner API | Internal only |
| lotus-miner | dynamic | TCP | libp2p P2P | Optional |
| curio | 12300 | TCP/HTTP | GUI Dashboard | Optional (localhost) |
| curio | 32100 | TCP/HTTP | Curio API | Internal only |
| curio | dynamic | TCP | libp2p P2P | Optional |
| yugabyte | 5433 | TCP | PostgreSQL (YSQL) | Internal only |
| yugabyte | 9000 | TCP/HTTP | Master UI | Optional (localhost) |
| yugabyte | 9100 | TCP/HTTP | TServer UI | Optional (localhost) |
| yugabyte | 7000 | TCP | Master RPC | Internal only |

---

## Security Considerations

### Production Deployment Recommendations

1. **API Token Protection**:
   - Never commit tokens to version control
   - Rotate tokens regularly
   - Use different tokens for different services

2. **Network Isolation**:
   - Use Docker networks to isolate services
   - Expose only necessary ports to host
   - Consider using reverse proxy for API access

3. **Database Security**:
   - Change default YugabyteDB password
   - Use TLS for database connections
   - Restrict database access to Curio only

4. **EthRPC Security**:
   - Enable rate limiting
   - Use API keys for access control
   - Monitor for abuse

5. **Firewall Rules**:
   ```bash
   # Allow only localhost for sensitive services
   iptables -A INPUT -p tcp --dport 1234 -s 127.0.0.1 -j ACCEPT
   iptables -A INPUT -p tcp --dport 2345 -j DROP
   iptables -A INPUT -p tcp --dport 5433 -j DROP
   ```

---

## Troubleshooting Network Issues

### Common Issues and Solutions

1. **Cannot connect to Lotus API**:
   ```bash
   # Check if Lotus is running and listening
   netstat -tulpn | grep 1234
   
   # Verify token and connection
   lotus auth api-info --perm admin
   ```

2. **Miner cannot reach Lotus**:
   ```bash
   # Verify FULLNODE_API_INFO is set correctly
   echo $FULLNODE_API_INFO
   
   # Test connection
   curl -X POST http://127.0.0.1:1234/rpc/v0 \
     -H "Authorization: Bearer <token>" \
     -d '{"jsonrpc":"2.0","method":"Filecoin.Version","params":[],"id":1}'
   ```

3. **Curio cannot connect to database**:
   ```bash
   # Test PostgreSQL connection
   psql -h 127.0.0.1 -p 5433 -U yugabyte -d yugabyte
   
   # Check YugabyteDB is running
   docker logs yugabyte
   ```

4. **libp2p peer connection issues**:
   ```bash
   # Check listening addresses
   lotus net listen
   
   # Verify peers
   lotus net peers
   
   # Check firewall
   sudo iptables -L -n | grep <port>
   ```

---

## References

- [Lotus Local Network Documentation](https://lotus.filecoin.io/lotus/developers/local-network/)
- [Lotus Ethereum RPC Configuration](https://lotus.filecoin.io/lotus/configure/ethereum-rpc/)
- [Curio Storage Documentation](https://docs.curiostorage.org/)
- [YugabyteDB Documentation](https://docs.yugabyte.com/)
- [Filecoin Specifications](https://spec.filecoin.io/)

---

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-11-18 | 1.0 | FOC Team | Initial networking architecture documentation |
