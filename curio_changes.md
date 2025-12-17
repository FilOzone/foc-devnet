# Curio Setup Refactoring Plan

## Current vs Required

**Current CurioStep** does basic container startup. **Missing**:
- Wait for chain height > 5
- Miner actor creation
- Cluster config in YugabyteDB
- Storage attachment via API
- PDP service setup
- Contract address loading

## Startup Sequence

## Startup Sequence

### 1. Wait for Lotus Chain (height > 5)
we can skip this

### 2. Create Miner Actor (if `.init.setup` missing)
Lotus-shed has already created a new miner before so we don't have to go through this process.

Get owner/worker/control addresses from `~/.foc-localnet/state/addresses.json`:
```bash
docker exec foc-lotus lotus-shed miner create \
  --deposit-margin-factor 1.01 <owner> <worker> <control> 2KiB
```

### 3. Initialize Cluster Config (if `.init.config` missing)
```bash
# Create cluster
docker exec foc-curio curio config new-cluster $newminer

# Set miner in base config
docker exec foc-curio bash -c \
  "curio config get base | sed 's/#Miners = \[\]/Miners = [\"$newminer\"]/' | curio config set --title base"

# Create PDP layer
docker exec foc-curio curio config create --title pdp-only << 'EOF'
[HTTP]
  Enable = true
  DomainName = "curio"
  ListenAddress = "0.0.0.0:80"
[Subsystems]
  EnableCommP = true
  EnableParkPiece = true
  EnablePDP = true
  EnableMoveStorage = true
  EnableDealMarket = true
  EnableWebGui = true
  GuiAddress = "0.0.0.0:4701"
EOF
```

### 4. Attach Storage (if `.init.curio` missing)
```bash
# Start temp node
docker exec -d <foc-curio-with-run-id> curio run --nosync --layers seal,post,pdp-only,gui
sleep 20

# Get container IP
curio_ip=$(docker inspect <foc-curio-with-run-id> --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}')

# Wait for API
until docker exec <foc-curio-with-run-id> curio cli --machine $curio_ip:12300 wait-api; do sleep 5; done

# Attach storage
docker exec <foc-curio-with-run-id> curio cli --machine $curio_ip:12300 \
  storage attach --init --seal --store /home/foc-user/.curio
```

### 5. Setup PDP Service (if `.init.pdp` missing)

Get private key from `~/.foc-localnet/state/addresses.json` (NOT from lotus wallet):
```bash
# Import private key via RPC
curl -X POST http://$curio_ip:4701/api/webrpc/v0 \
  -d '{"jsonrpc":"2.0","method":"CurioWeb.ImportPDPKey","params":["<hex-key-from-addresses.json>"],"id":1}'

# Generate PDP keypair
docker exec <foc-curio-with-run-id> pdptool create-service-secret > pdp_key.txt
pub_key=$(sed -n '/Public Key:/,/-----END PUBLIC KEY-----/p' pdp_key.txt | grep -v "Public Key:")

# Register PDP service
curl -X POST http://$curio_ip:4701/api/webrpc/v0 \
  -d '{"jsonrpc":"2.0","method":"CurioWeb.AddPDPService","params":["pdp","<escaped-pub-key>"],"id":2}'

# Generate JWT token
docker exec <foc-curio-with-run-id> pdptool create-jwt-token pdp > jwt_token.txt
```

### 7. Start Production Node
```bash
curio run --nosync --name devnet --layers seal,post,pdp-only,gui
```

## Module Structure

```
src/commands/start/curio/
├── mod.rs              # Exports and orchestration
├── step.rs             # Main CurioStep implementation
├── constants.rs        # Magic numbers/strings
├── chain.rs            # Wait for Lotus chain readiness
├── miner.rs            # Miner actor creation
├── cluster.rs          # Cluster config in YugabyteDB
├── storage.rs          # Storage attachment
├── pdp.rs              # PDP setup orchestration
├── contracts.rs        # Load contract addresses
└── docker.rs           # Docker command building
```

## Implementation Steps

### constants.rs
```rust
pub const LOTUS_MIN_HEIGHT: u64 = 5;
pub const CURIO_API_PORT: u16 = 12300;
pub const CURIO_GUI_PORT: u16 = 4701;
pub const MINER_SECTOR_SIZE: &str = "2KiB";
pub const MARKER_SETUP: &str = ".init.setup";
pub const MARKER_CONFIG: &str = ".init.config";
pub const MARKER_CURIO: &str = ".init.curio";
pub const MARKER_PDP: &str = ".init.pdp";
```

### chain.rs
```rust
/// Wait for Lotus chain height > min_height
pub fn wait_for_chain_ready(lotus_container: &str, min_height: u64) -> Result<()>

/// Get current chain height
fn get_chain_height(lotus_container: &str) -> Result<u64>
```

### miner.rs
```rust
/// Create miner actor if marker missing. Returns miner ID.
pub fn ensure_miner_created(
    lotus_container: &str,
    addresses: &AddressesJson,  // From ~/.foc-localnet/state/addresses.json
    marker_dir: &Path
) -> Result<String>

/// Get new miner ID (exclude t01000, t01001)
fn get_new_miner_id(lotus_container: &str) -> Result<String>
```

### cluster.rs
```rust
/// Initialize cluster config if marker missing
pub fn ensure_cluster_initialized(
    curio_container: &str,
    miner_id: &str,
    marker_dir: &Path
) -> Result<()>

/// Create PDP config layer
fn create_pdp_layer(curio_container: &str) -> Result<()>
```

### storage.rs
```rust
/// Attach storage if marker missing
pub fn ensure_storage_attached(
    curio_container: &str,
    storage_path: &str,
    marker_dir: &Path
) -> Result<()>

/// Start/stop temp Curio node
fn start_temp_node(container: &str) -> Result<()>
fn stop_temp_node(container: &str) -> Result<()>
```

### pdp.rs
```rust
/// Setup PDP service if marker missing
pub fn ensure_pdp_setup(
    curio_container: &str,
    curio_ip: &str,
    addresses: &AddressesJson,  // For private key
    marker_dir: &Path
) -> Result<()>

/// Import private key from addresses.json
fn import_private_key(curio_ip: &str, key_hex: &str) -> Result<()>

/// Register PDP service
fn register_pdp_service(curio_ip: &str, pub_key: &str) -> Result<()>

/// Generate JWT token
fn create_jwt_token(curio_container: &str) -> Result<String>
```

### contracts.rs
```rust
/// Load contract addresses from foc-contract-addresses.json
pub fn load_contract_addresses(
    addresses_file: &Path
) -> Result<HashMap<String, String>>

/// Build env vars for Curio container
pub fn build_contract_env_vars(addresses: &HashMap<String, String>) -> Vec<String>
```

### step.rs
```rust
impl Step for CurioStep {
    fn pre_execute(&self, ctx: &mut StepContext) -> Result<()> {
        // Check Lotus, YugabyteDB running
        // Wait for chain height > 5
        chain::wait_for_chain_ready("foc-lotus", LOTUS_MIN_HEIGHT)?;
    }

    fn execute(&self, ctx: &mut StepContext) -> Result<()> {
        let addresses = load_addresses_json()?; // ~/.foc-localnet/state/addresses.json
        let marker_dir = self.volumes_dir.join("curio/.curio");

        // 1. Create miner actor
        let miner_id = miner::ensure_miner_created("foc-lotus", &addresses, &marker_dir)?;

        // 2. Start container (basic)
        let container_id = docker::start_curio_container(ctx, &self.volumes_dir)?;

        // 3. Initialize cluster
        cluster::ensure_cluster_initialized("foc-curio", &miner_id, &marker_dir)?;

        // 4. Attach storage
        storage::ensure_storage_attached("foc-curio", "/home/foc-user/.curio", &marker_dir)?;

        // 5. Setup PDP
        let curio_ip = get_container_ip("foc-curio")?;
        pdp::ensure_pdp_setup("foc-curio", &curio_ip, &addresses, &marker_dir)?;

        // 6. Load contracts and restart with env vars
        let contract_addrs = contracts::load_contract_addresses(
            &contract_addresses_file()
        )?;
        let env_vars = contracts::build_contract_env_vars(&contract_addrs);
        
        // Restart container with contract env vars
        docker::restart_with_env_vars("foc-curio", &env_vars)?;

        Ok(())
    }

    fn post_execute(&self, ctx: &mut StepContext) -> Result<()> {
        // Verify container running, API responsive, PDP pingable
    }
}
```

## Key Simplifications

1. **Never use lotus wallet commands** - All keys in `~/.foc-localnet/state/addresses.json`
2. **Marker files for idempotency** - Skip completed steps on restart
3. **Single module per concern** - No nested sub-modules
4. **Minimal orchestration** - step.rs calls sub-modules in order
5. **No complex bash** - All logic in Rust, simple docker exec calls