#!/bin/bash

# Manual Curio Setup Script for foc-localnet
# This script replicates what CurioStep does, allowing manual debugging
# Run this after Lotus, Lotus-Miner, FOC Deploy, and Yugabyte are running

set -e  # Exit on any error

# Configuration - adjust these as needed
FOC_HOME="${FOC_HOME:-$HOME/.foc-localnet}"
RUN_ID="${RUN_ID:-dev}"  # Use a run ID for container naming
LOTUS_CONTAINER="foc-lotus-${RUN_ID}"
YUGABYTE_CONTAINER="foc-yugabyte-${RUN_ID}"
CURIO_CONTAINER="foc-curio-${RUN_ID}"
CURIO_NETWORK="foc-curio-${RUN_ID}"
FILECOIN_NETWORK="foc-filecoin-${RUN_ID}"

# Ports - these should match what's allocated in the startup sequence
CURIO_API_PORT="${CURIO_API_PORT:-12300}"
CURIO_RPC_PORT="${CURIO_RPC_PORT:-12301}"
CURIO_GUI_PORT="${CURIO_GUI_PORT:-4701}"
CURIO_HTTP_PORT="${CURIO_HTTP_PORT:-4702}"

# Miner ID from genesis
CURIO_MINER_ID="t01000"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${GREEN}INFO:${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}WARN:${NC} $1"
}

echo_error() {
    echo -e "${RED}ERROR:${NC} $1"
}

# Check if container exists and is running
check_container() {
    local container_name=$1
    if docker ps --format "table {{.Names}}" | grep -q "^${container_name}$"; then
        echo_info "Container $container_name is running"
        return 0
    elif docker ps -a --format "table {{.Names}}" | grep -q "^${container_name}$"; then
        echo_warn "Container $container_name exists but is not running"
        return 1
    else
        echo_error "Container $container_name does not exist"
        return 2
    fi
}

# Clean up existing container
cleanup_container() {
    local container_name=$1
    if docker ps -a --format "table {{.Names}}" | grep -q "^${container_name}$"; then
        echo_info "Removing existing container $container_name"
        docker stop "$container_name" 2>/dev/null || true
        docker rm "$container_name" 2>/dev/null || true
    fi
}

# Get container IP
get_container_ip() {
    local container_name=$1
    docker inspect "$container_name" --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}'
}

# Wait for port to be available
wait_for_port() {
    local host=$1
    local port=$2
    local timeout=${3:-30}
    echo_info "Waiting for $host:$port to be available (timeout: ${timeout}s)"
    for i in $(seq 1 $timeout); do
        if nc -z "$host" "$port" 2>/dev/null; then
            echo_info "Port $host:$port is now available"
            return 0
        fi
        sleep 1
    done
    echo_error "Port $host:$port did not become available within ${timeout}s"
    return 1
}

# Load contract addresses
load_contract_addresses() {
    local contract_file="$FOC_HOME/foc-contract-addresses.json"
    if [ -f "$contract_file" ]; then
        echo_info "Loading contract addresses from $contract_file"
        # Export as environment variables for Curio
        while IFS=':' read -r key value; do
            key=$(echo "$key" | tr -d ' "')
            value=$(echo "$value" | tr -d ' ",')
            if [ -n "$key" ] && [ -n "$value" ]; then
                export "$key=$value"
                echo_info "  $key=$value"
            fi
        done < <(jq -r 'to_entries[] | "\(.key):\(.value)"' "$contract_file")
    else
        echo_warn "Contract addresses file not found: $contract_file"
    fi
}

# Load addresses from state
load_state_addresses() {
    local state_file="$FOC_HOME/state/addresses.json"
    if [ ! -f "$state_file" ]; then
        echo_error "State addresses file not found: $state_file"
        return 1
    fi

    echo_info "Loading state addresses from $state_file"

    # Extract FEVM_FAUCET private key for PDP
    PDP_PRIVATE_KEY=$(jq -r '.[] | select(.name == "FEVM_FAUCET") | .private_key' "$state_file")
    if [ -z "$PDP_PRIVATE_KEY" ] || [ "$PDP_PRIVATE_KEY" = "null" ]; then
        echo_error "FEVM_FAUCET private key not found in state addresses"
        return 1
    fi
    echo_info "Found FEVM_FAUCET private key for PDP operations"
}

# Read Lotus API token
read_lotus_token() {
    local token_file="$FOC_HOME/artifacts/docker/volumes/lotus-data/token"
    if [ ! -f "$token_file" ]; then
        echo_error "Lotus token file not found: $token_file"
        return 1
    fi
    cat "$token_file"
}

# Main setup function
setup_curio() {
    echo_info "Starting manual Curio setup..."

    # Check dependencies
    echo_info "Checking dependencies..."
    check_container "$LOTUS_CONTAINER"
    if [ $? -ne 0 ]; then
        echo_error "Lotus container is not running. Please start Lotus first."
        exit 1
    fi

    check_container "$YUGABYTE_CONTAINER"
    if [ $? -ne 0 ]; then
        echo_error "YugabyteDB container is not running. Please start YugabyteDB first."
        exit 1
    fi

    # Load configuration
    load_contract_addresses
    load_state_addresses

    LOTUS_TOKEN=$(read_lotus_token)
    LOTUS_API_INFO="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJBbGxvdyI6WyJyZWFkIiwid3JpdGUiLCJzaWduIiwiYWRtaW4iXX0.${LOTUS_TOKEN}:http://${LOTUS_CONTAINER}:1234/rpc/v1"

    # Clean up any existing Curio container
    cleanup_container "$CURIO_CONTAINER"

    # Create data directories
    echo_info "Creating Curio data directories..."
    mkdir -p "$FOC_HOME/artifacts/docker/volumes/curio/.curio"
    mkdir -p "$FOC_HOME/artifacts/docker/volumes/curio/fast-storage"
    mkdir -p "$FOC_HOME/artifacts/docker/volumes/curio/long-term-storage"

    # Build Docker run command
    echo_info "Starting Curio container..."

    docker run -d \
        --name "$CURIO_CONTAINER" \
        --network "$CURIO_NETWORK" \
        -p "$CURIO_API_PORT:12300" \
        -p "$CURIO_RPC_PORT:12301" \
        -p "$CURIO_GUI_PORT:4701" \
        -p "$CURIO_HTTP_PORT:4702" \
        -v "$FOC_HOME/artifacts/docker/volumes/curio/.curio:/home/foc-user/.curio" \
        -v "$FOC_HOME/artifacts/docker/volumes/curio/fast-storage:/home/foc-user/curio/fast-storage" \
        -v "$FOC_HOME/artifacts/docker/volumes/curio/long-term-storage:/home/foc-user/curio/long-term-storage" \
        -v "$FOC_HOME/artifacts/bin/curio:/usr/local/bin/lotus-bins/curio" \
        -v "$FOC_HOME/artifacts/docker/volumes/lotus-data:/home/foc-user/.lotus-local-net" \
        -v "$FOC_HOME/artifacts/docker/volumes/genesis-sectors/curio-miner:/sectors" \
        -v "$FOC_HOME/artifacts/docker/volumes/foc-builder/cargo:/cargo" \
        -e "CURIO_DB_HOST=$YUGABYTE_CONTAINER" \
        -e "CURIO_DB_PORT=5433" \
        -e "CURIO_DB_USER=yugabyte" \
        -e "CURIO_DB_PASSWORD=yugabyte" \
        -e "CURIO_DB_NAME=yugabyte" \
        -e "CURIO_DB_LOAD_BALANCE=false" \
        -e "LOTUS_API=http://${LOTUS_CONTAINER}:1234/rpc/v1" \
        -e "FULLNODE_API_INFO=$LOTUS_API_INFO" \
        -e "LOTUS_PATH=/home/foc-user/.lotus-local-net" \
        ${MOCKUSDFC:+MOCKUSDFC=$MOCKUSDFC} \
        ${FOC_SERVICE_PROVIDER_REGISTRY:+FOC_SERVICE_PROVIDER_REGISTRY=$FOC_SERVICE_PROVIDER_REGISTRY} \
        ${FILECOIN_WARM_STORAGE:+FILECOIN_WARM_STORAGE=$FILECOIN_WARM_STORAGE} \
        ${PDP_VERIFIER:+PDP_VERIFIER=$PDP_VERIFIER} \
        foc-curio \
        sleep infinity

    # Connect to filecoin network for Lotus access
    echo_info "Connecting Curio to filecoin network..."
    docker network connect "$FILECOIN_NETWORK" "$CURIO_CONTAINER" 2>/dev/null || true

    # Wait for container to be ready
    echo_info "Waiting for container to initialize..."
    sleep 5

    # Get container IP for API calls
    CURIO_IP=$(get_container_ip "$CURIO_CONTAINER")
    echo_info "Curio container IP: $CURIO_IP"

    # Initialize cluster if not already done
    if [ ! -f "$FOC_HOME/artifacts/docker/volumes/curio/.curio/.init.config" ]; then
        echo_info "Initializing cluster configuration..."

        # Create new cluster
        echo_info "Creating new cluster for miner $CURIO_MINER_ID..."
        docker exec "$CURIO_CONTAINER" /usr/local/bin/lotus-bins/curio config new-cluster "$CURIO_MINER_ID"

        # Set miner in base config
        echo_info "Setting miner ID in base config..."
        docker exec "$CURIO_CONTAINER" bash -c \
            "/usr/local/bin/lotus-bins/curio config get base | sed 's/#Miners = \[\]/Miners = [\"$CURIO_MINER_ID\"]/' | /usr/local/bin/lotus-bins/curio config set --title base"

        # Create PDP config layer
        echo_info "Creating PDP config layer..."
        docker exec "$CURIO_CONTAINER" /usr/local/bin/lotus-bins/curio config create --title pdp-only << 'EOF'
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

        # Mark cluster config as done
        touch "$FOC_HOME/artifacts/docker/volumes/curio/.curio/.init.config"
        echo_info "Cluster configuration completed"
    else
        echo_info "Cluster configuration already done, skipping..."
    fi

    # Attach storage if not already done
    if [ ! -f "$FOC_HOME/artifacts/docker/volumes/curio/.curio/.init.curio" ]; then
        echo_info "Attaching storage..."

        # Start temporary Curio daemon for storage attachment
        echo_info "Starting temporary Curio daemon..."
        docker exec -d "$CURIO_CONTAINER" /usr/local/bin/lotus-bins/curio run --nosync --layers seal,post,pdp-only,gui

        echo_info "Waiting for Curio daemon to initialize..."
        sleep 25

        # Wait for API to be ready
        echo_info "Waiting for Curio API..."
        for i in {1..12}; do
            if docker exec "$CURIO_CONTAINER" curl -s "http://localhost:$CURIO_GUI_PORT/api/webrpc/v0" > /dev/null 2>&1; then
                echo_info "Curio API is ready"
                break
            fi
            echo_info "Waiting for API (attempt $i/12)..."
            sleep 5
        done

        # Attach storage paths
        echo_info "Attaching fast storage..."
        docker exec "$CURIO_CONTAINER" /usr/local/bin/lotus-bins/curio cli storage attach --init --seal /home/foc-user/curio/fast-storage

        echo_info "Attaching long-term storage..."
        docker exec "$CURIO_CONTAINER" /usr/local/bin/lotus-bins/curio cli storage attach --init --store /home/foc-user/curio/long-term-storage

        # Stop temporary daemon
        echo_info "Stopping temporary daemon..."
        docker exec "$CURIO_CONTAINER" pkill -f curio || true
        sleep 5

        # Mark storage attachment as done
        touch "$FOC_HOME/artifacts/docker/volumes/curio/.curio/.init.curio"
        echo_info "Storage attachment completed"
    else
        echo_info "Storage already attached, skipping..."
    fi

    # Setup PDP service if not already done
    if [ ! -f "$FOC_HOME/artifacts/docker/volumes/curio/.curio/.init.pdp" ]; then
        echo_info "Setting up PDP service..."

        # Start temporary daemon again for PDP setup
        echo_info "Starting temporary daemon for PDP setup..."
        docker exec -d "$CURIO_CONTAINER" /usr/local/bin/lotus-bins/curio run --nosync --layers seal,post,pdp-only,gui
        sleep 25

        # Import private key via WebRPC
        echo_info "Importing private key for PDP..."
        curl -X POST "http://$CURIO_IP:$CURIO_GUI_PORT/api/webrpc/v0" \
            -H "Content-Type: application/json" \
            -d "{\"jsonrpc\":\"2.0\",\"method\":\"CurioWeb.ImportPDPKey\",\"params\":[\"$PDP_PRIVATE_KEY\"],\"id\":1}"

        # Generate PDP keypair
        echo_info "Generating PDP keypair..."
        docker exec "$CURIO_CONTAINER" pdptool create-service-secret > /tmp/pdp_key.txt
        PDP_PUB_KEY=$(sed -n '/-----BEGIN PUBLIC KEY-----/,/-----END PUBLIC KEY-----/p' /tmp/pdp_key.txt | tr -d '\n')

        # Register PDP service
        echo_info "Registering PDP service..."
        ESCAPED_PUB_KEY=$(echo "$PDP_PUB_KEY" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\n/\\n/g')
        curl -X POST "http://$CURIO_IP:$CURIO_GUI_PORT/api/webrpc/v0" \
            -H "Content-Type: application/json" \
            -d "{\"jsonrpc\":\"2.0\",\"method\":\"CurioWeb.AddPDPService\",\"params\":[\"pdp\",\"$ESCAPED_PUB_KEY\"],\"id\":2}"

        # Generate JWT token
        echo_info "Generating JWT token..."
        docker exec "$CURIO_CONTAINER" pdptool create-jwt-token pdp > /tmp/jwt_token.txt

        # Stop temporary daemon
        echo_info "Stopping temporary daemon..."
        docker exec "$CURIO_CONTAINER" pkill -f curio || true
        sleep 5

        # Mark PDP setup as done
        touch "$FOC_HOME/artifacts/docker/volumes/curio/.curio/.init.pdp"
        echo_info "PDP service setup completed"
    else
        echo_info "PDP service already set up, skipping..."
    fi

    # Start final production daemon
    echo_info "Starting final Curio daemon..."
    docker exec -d "$CURIO_CONTAINER" /usr/local/bin/lotus-bins/curio run --nosync --name devnet --layers seal,post,pdp-only,gui

    # Wait for daemon to start
    echo_info "Waiting for Curio daemon to initialize..."
    sleep 10

    # Verify ports are accessible
    echo_info "Verifying port accessibility..."
    wait_for_port "localhost" "$CURIO_API_PORT" 30 || echo_warn "API port $CURIO_API_PORT may not be immediately available"
    wait_for_port "localhost" "$CURIO_RPC_PORT" 30 || echo_warn "RPC port $CURIO_RPC_PORT may not be immediately available"
    wait_for_port "localhost" "$CURIO_GUI_PORT" 30 || echo_warn "GUI port $CURIO_GUI_PORT may not be immediately available"
    wait_for_port "localhost" "$CURIO_HTTP_PORT" 30 || echo_warn "HTTP port $CURIO_HTTP_PORT may not be immediately available"

    # Check if Curio is responsive
    echo_info "Checking Curio responsiveness..."
    if docker exec "$CURIO_CONTAINER" /usr/local/bin/lotus-bins/curio version > /dev/null 2>&1; then
        echo_info "Curio is responding to commands"
    else
        echo_warn "Curio may not be fully responsive yet"
    fi

    echo_info "Curio setup completed!"
    echo_info "  API endpoint: http://localhost:$CURIO_API_PORT"
    echo_info "  RPC endpoint: http://localhost:$CURIO_RPC_PORT"
    echo_info "  GUI: http://localhost:$CURIO_GUI_PORT"
    echo_info "  HTTP PDP: http://localhost:$CURIO_HTTP_PORT"
}

# Main execution
case "${1:-}" in
    "setup")
        setup_curio
        ;;
    "cleanup")
        cleanup_container "$CURIO_CONTAINER"
        echo_info "Cleanup completed"
        ;;
    "status")
        check_container "$CURIO_CONTAINER"
        ;;
    "logs")
        docker logs "$CURIO_CONTAINER" ${2:---tail 50}
        ;;
    "shell")
        docker exec -it "$CURIO_CONTAINER" /bin/bash
        ;;
    *)
        echo "Usage: $0 {setup|cleanup|status|logs|shell}"
        echo ""
        echo "Commands:"
        echo "  setup   - Run the complete Curio setup process"
        echo "  cleanup - Remove existing Curio container"
        echo "  status  - Check if Curio container is running"
        echo "  logs    - Show Curio container logs (add --follow for tail)"
        echo "  shell   - Open shell in Curio container"
        echo ""
        echo "Environment variables:"
        echo "  FOC_HOME     - foc-localnet home directory (default: ~/.foc-localnet)"
        echo "  RUN_ID       - Run ID for container naming (default: dev)"
        echo "  CURIO_API_PORT  - API port (default: 12300)"
        echo "  CURIO_RPC_PORT  - RPC port (default: 12301)"
        echo "  CURIO_GUI_PORT  - GUI port (default: 4701)"
        echo "  CURIO_HTTP_PORT - HTTP port (default: 4702)"
        exit 1
        ;;
esac</content>
<parameter name="filePath">/home/redpanda/Projects/foc-localnet/manual_curio_setup.sh