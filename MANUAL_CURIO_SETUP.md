# Manual Curio Setup Guide

This guide provides step-by-step commands to manually set up Curio in foc-localnet, replicating what `CurioStep` does. Run these commands in order after ensuring all prerequisites are met.

## Prerequisites

Before starting, ensure these services are running:
- Lotus daemon
- Lotus miner
- FOC contracts deployed
- YugabyteDB

Check with:
```bash
docker ps | grep -E "(foc-lotus|foc-lotus-miner|foc-yugabyte)"
```

## Configuration

Set these environment variables (adjust as needed):
```bash
export FOC_HOME="${FOC_HOME:-$HOME/.foc-localnet}"
echo "FOC_HOME=$FOC_HOME"
export RUN_ID="${RUN_ID:-dev}"
echo "RUN_ID=$RUN_ID"
export LOTUS_CONTAINER="foc-lotus-${RUN_ID}"
echo "LOTUS_CONTAINER=$LOTUS_CONTAINER"
export YUGABYTE_CONTAINER="foc-yugabyte-${RUN_ID}"
echo "YUGABYTE_CONTAINER=$YUGABYTE_CONTAINER"
export CURIO_CONTAINER="foc-curio-${RUN_ID}"
echo "CURIO_CONTAINER=$CURIO_CONTAINER"
export CURIO_NETWORK="foc-curio-${RUN_ID}"
echo "CURIO_NETWORK=$CURIO_NETWORK"
export FILECOIN_NETWORK="foc-filecoin-${RUN_ID}"
echo "FILECOIN_NETWORK=$FILECOIN_NETWORK"
export CURIO_API_PORT="${CURIO_API_PORT:-12300}"
echo "CURIO_API_PORT=$CURIO_API_PORT"
export CURIO_RPC_PORT="${CURIO_RPC_PORT:-12301}"
echo "CURIO_RPC_PORT=$CURIO_RPC_PORT"
export CURIO_GUI_PORT="${CURIO_GUI_PORT:-4701}"
echo "CURIO_GUI_PORT=$CURIO_GUI_PORT"
export CURIO_HTTP_PORT="${CURIO_HTTP_PORT:-4702}"
echo "CURIO_HTTP_PORT=$CURIO_HTTP_PORT"
export CURIO_MINER_ID="t01001"
echo "CURIO_MINER_ID=$CURIO_MINER_ID"
```

## Step 1: Load Configuration

Load contract addresses and state information:
```bash
# Load contract addresses as environment variables
if [ -f "$HOME/.foc-localnet/foc-contract-addresses.json" ]; then
    eval "$(jq -r 'to_entries[] | "export \(.key)=\(.value)"' "$HOME/.foc-localnet/foc-contract-addresses.json")"
fi

# Load PDP private key
PDP_PRIVATE_KEY=$(jq -r '.[] | select(.name == "FEVM_FAUCET") | .private_key' "$HOME/.foc-localnet/state/addresses.json")

# Get Lotus token
LOTUS_TOKEN=$(cat "$HOME/.foc-localnet/artifacts/docker/volumes/lotus-data/token")
LOTUS_API_INFO="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJBbGxvdyI6WyJyZWFkIiwid3JpdGUiLCJzaWduIiwiYWRtaW4iXX0.${LOTUS_TOKEN}:http://foc-lotus-dev:1234/rpc/v1"
```

## Step 2: Clean Up and Prepare

Remove any existing Curio container and create directories:
```bash
# Stop and remove existing container
docker stop "foc-curio-dev" 2>/dev/null || true
docker rm "foc-curio-dev" 2>/dev/null || true

# Create data directories
mkdir -p "$HOME/.foc-localnet/artifacts/docker/volumes/curio/.curio"
mkdir -p "$HOME/.foc-localnet/artifacts/docker/volumes/curio/fast-storage"
mkdir -p "$HOME/.foc-localnet/artifacts/docker/volumes/curio/long-term-storage"
```

## Step 3: Start Curio Container

Start the container in sleep mode for initialization:
```bash
docker run -d \
    --name "foc-curio-dev" \
    --network "foc-curio-dev" \
    -p "12300:12300" \
    -p "12301:12301" \
    -p "4701:4701" \
    -p "4702:4702" \
    -v "$HOME/.foc-localnet/artifacts/docker/volumes/curio/.curio:/home/foc-user/.curio" \
    -v "$HOME/.foc-localnet/artifacts/docker/volumes/curio/fast-storage:/home/foc-user/curio/fast-storage" \
    -v "$HOME/.foc-localnet/artifacts/docker/volumes/curio/long-term-storage:/home/foc-user/curio/long-term-storage" \
    -v "$HOME/.foc-localnet/artifacts/bin/curio:/usr/local/bin/lotus-bins/curio" \
    -v "$HOME/.foc-localnet/artifacts/docker/volumes/lotus-data:/home/foc-user/.lotus-local-net" \
    -v "$HOME/.foc-localnet/artifacts/docker/volumes/genesis-sectors/curio-miner:/sectors" \
    -v "$HOME/.foc-localnet/artifacts/docker/volumes/foc-builder/cargo:/cargo" \
    -e "CURIO_DB_HOST=foc-yugabyte-dev" \
    -e "CURIO_DB_PORT=5433" \
    -e "CURIO_DB_USER=yugabyte" \
    -e "CURIO_DB_PASSWORD=yugabyte" \
    -e "CURIO_DB_NAME=yugabyte" \
    -e "CURIO_DB_LOAD_BALANCE=false" \
    -e "LOTUS_API=http://foc-lotus-dev:1234/rpc/v1" \
    -e "FULLNODE_API_INFO=$LOTUS_API_INFO" \
    -e "LOTUS_PATH=/home/foc-user/.lotus-local-net" \
    foc-curio \
    sleep infinity 
```

Connect to filecoin network:
```bash
docker network connect "foc-filecoin-dev" "foc-curio-dev" 2>/dev/null || true
```

## Step 4: Initialize Cluster Configuration

Only run if `.init.config` marker doesn't exist:
```bash
# Check if already done
if [ ! -f "$HOME/.foc-localnet/artifacts/docker/volumes/curio/.curio/.init.config" ]; then
    # Create new cluster
    docker exec "foc-curio-dev" /usr/local/bin/lotus-bins/curio config new-cluster "t01001"

    # Set miner in base config
    docker exec "foc-curio-dev" bash -c \
        "/usr/local/bin/lotus-bins/curio config get base | sed 's/#Miners = \[\]/Miners = [\"t01001\"]/' | /usr/local/bin/lotus-bins/curio config set --title base"

    # Create PDP config layer
    docker exec "foc-curio-dev" /usr/local/bin/lotus-bins/curio config create --title pdp-only << 'EOF'
[HTTP]
DelegateTLS = true
DomainName = "pdp-sp-0.foc-localnet.internal"
Enable = true
ListenAddress = "0.0.0.0:4702"

[Subsystems]
EnableCommP = true
EnableMoveStorage = true
EnablePDP = true
EnableParkPiece = true
EOF

    # Mark as done
    touch "$HOME/.foc-localnet/artifacts/docker/volumes/curio/.curio/.init.config"
    echo "Cluster configuration completed"
else
    echo "Cluster configuration already done, skipping..."
fi
```

## Step 5: Attach Storage

Only run if `.init.curio` marker doesn't exist:
```bash
# Check if already done
if [ ! -f "$HOME/.foc-localnet/artifacts/docker/volumes/curio/.curio/.init.curio" ]; then
    # Start temporary daemon
    docker exec -d "foc-curio-dev" /usr/local/bin/lotus-bins/curio run --nosync --layers seal,post,pdp-only,gui

    # Wait for daemon
    echo "Waiting for Curio daemon to initialize..."
    sleep 25

    # Wait for API
    echo "Waiting for Curio API..."
    for i in {1..12}; do
        if docker exec "foc-curio-dev" curl -s "http://localhost:4701/api/webrpc/v0" > /dev/null 2>&1; then
            echo "Curio API is ready"
            break
        fi
        echo "Waiting for API (attempt $i/12)..."
        sleep 5
    done

    # Attach storage
    docker exec "foc-curio-dev" /usr/local/bin/lotus-bins/curio cli storage attach --init --seal /home/foc-user/curio/fast-storage
    docker exec "foc-curio-dev" /usr/local/bin/lotus-bins/curio cli storage attach --init --store /home/foc-user/curio/long-term-storage

    # Stop temporary daemon
    docker exec "foc-curio-dev" pkill -f curio || true
    sleep 5

    # Mark as done
    touch "$HOME/.foc-localnet/artifacts/docker/volumes/curio/.curio/.init.curio"
    echo "Storage attachment completed"
else
    echo "Storage already attached, skipping..."
fi
```

## Step 6: Setup PDP Service

Only run if `.init.pdp` marker doesn't exist:
```bash
# Check if already done
if [ ! -f "$HOME/.foc-localnet/artifacts/docker/volumes/curio/.curio/.init.pdp" ]; then
    # Get container IP
    CURIO_IP=$(docker inspect "foc-curio-dev" --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}')

    # Start temporary daemon
    docker exec -d "foc-curio-dev" /usr/local/bin/lotus-bins/curio run --nosync --layers seal,post,pdp-only,gui
    sleep 25

    # Import private key
    curl -X POST "http://$CURIO_IP:4701/api/webrpc/v0" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"CurioWeb.ImportPDPKey\",\"params\":[\"$PDP_PRIVATE_KEY\"],\"id\":1}"

    # Generate PDP keypair
    docker exec "foc-curio-dev" pdptool create-service-secret > /tmp/pdp_key.txt
    PDP_PUB_KEY=$(sed -n '/-----BEGIN PUBLIC KEY-----/,/-----END PUBLIC KEY-----/p' /tmp/pdp_key.txt | tr -d '\n')

    # Register PDP service
    ESCAPED_PUB_KEY=$(echo "$PDP_PUB_KEY" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\n/\\n/g')
    curl -X POST "http://$CURIO_IP:4701/api/webrpc/v0" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"CurioWeb.AddPDPService\",\"params\":[\"pdp\",\"$ESCAPED_PUB_KEY\"],\"id\":2}"

    # Generate JWT token
    docker exec "foc-curio-dev" pdptool create-jwt-token pdp > /tmp/jwt_token.txt

    # Stop temporary daemon
    docker exec "foc-curio-dev" pkill -f curio || true
    sleep 5

    # Mark as done
    touch "$HOME/.foc-localnet/artifacts/docker/volumes/curio/.curio/.init.pdp"
    echo "PDP service setup completed"
else
    echo "PDP service already set up, skipping..."
fi
```

## Step 7: Start Final Daemon

Start the production Curio daemon:
```bash
# Start final daemon
docker exec -d "foc-curio-dev" /usr/local/bin/lotus-bins/curio run --nosync --name devnet --layers seal,post,pdp-only,gui

# Wait for startup
echo "Waiting for Curio daemon to initialize..."
sleep 10
```

## Step 8: Verify Setup

Check that everything is working:
```bash
# Check container is running
docker ps | grep "foc-curio-dev"

# Check ports
netstat -tlnp | grep -E "(:12300|:12301|:4701|:4702)"

# Test Curio responsiveness
docker exec "foc-curio-dev" /usr/local/bin/lotus-bins/curio version

# View logs
docker logs "foc-curio-dev" --tail 20
```

## Access Points

After successful setup, Curio will be available at:
- **API**: http://localhost:12300
- **RPC**: http://localhost:12301
- **GUI**: http://localhost:4701
- **HTTP PDP**: http://localhost:4702

## Debugging Commands

```bash
# View logs
docker logs "foc-curio-dev" --tail 50 --follow

# Enter container
docker exec -it "foc-curio-dev" /bin/bash

# Check Curio status
docker exec "foc-curio-dev" ps aux | grep curio

# Check database connection
docker exec "foc-curio-dev" /usr/local/bin/lotus-bins/curio config get

# Test API
curl -s "http://localhost:4701/api/webrpc/v0" | head -c 100
```

## Cleanup

To start over:
```bash
docker stop "foc-curio-dev"
docker rm "foc-curio-dev"
rm -rf "$HOME/.foc-localnet/artifacts/docker/volumes/curio"
```</content>
<parameter name="filePath">/home/redpanda/Projects/foc-localnet/MANUAL_CURIO_SETUP.md