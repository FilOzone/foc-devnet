# foc-devnet

**Run a local Filecoin network with FOC (Filecoin Onchain Contracts) in minutes.**

A developer-friendly tool for spinning up complete Filecoin test networks with smart contract support, deterministic key generation, and automated deployment—all running locally in Docker.

---

## 🚀 Quick Start

Get up and running in three simple steps:

### Prerequisites

**Non-root user with Docker access**: `foc-devnet` must be run by a non-root user in the `docker` group.

```bash
echo $(id -u); groups | grep 'docker'
```

**Configure host.docker.internal**: Add this entry to `/etc/hosts` so SP URLs work from both host and containers:

```bash
echo '127.0.0.1 host.docker.internal' | sudo tee -a /etc/hosts
```

This is required for SP-to-SP fetch. `foc-devnet start` will check for this and fail with instructions if not configured.

For GitHub Actions, add this step before running foc-devnet:
```yaml
- run: echo '127.0.0.1 host.docker.internal' | sudo tee -a /etc/hosts
```

**Linting (rust + python)**:
```sh
sudo apt install pipx && pipx ensurepath && pipx install black
./scripts/lint.sh        # auto-fix mode (default)
FIX=0 ./scripts/lint.sh # check-only mode (used in CI)
```

**Pre-commit hook** (optional — runs `lint.sh` before each commit):
```sh
./scripts/install_precommit_hooks.sh
```

### Step 1: Initialize

```bash
cargo run -- init
```

This will:
- Download required repositories (or use your local ones)
- Build Docker images
- Generate deterministic cryptographic keys
- Prepare the environment

**Using local repositories?** Specify them during init:

```bash
cargo run -- init \
    --curio local:/home/user/code/curio \
    --filecoin-services local:/home/user/code/filecoin-services \
    --lotus local:/home/user/code/lotus \
```

**Re-initializing?** Run `cargo run -- clean` first to remove existing state. Config is preserved by default; use `--all` to reset it too.

### Step 2: Build

```bash
cargo run -- build lotus
cargo run -- build curio
```

This will:
- Compile Lotus and Curio binaries inside Docker containers
- Cache build artifacts for faster subsequent builds
- You can run them in parallel as well for faster builds, if you have a powerful PC.

### Step 3: Start the Network

```bash
cargo run -- start --parallel
```

This will:
- Create the genesis block
- Start Lotus daemon with FEVM enabled
- Deploy FOC smart contracts (including MockUSDFC)
- Start storage provider(s)
- Launch [Portainer UI](https://docs.docksal.io/use-cases/portainer/) for container management

**If you have troubles**: Use `cargo run -- start`, removing parallelism during start, this may take longer.

**That's it!** Your local Filecoin network is running.

### Step 4: Use the Network

After startup, `USER_1` is fully configured for FOC: USDFC deposited into FilecoinPay and FWSS approved as an operator. You can use it immediately with synapse-sdk:

```bash
NETWORK=devnet node utils/example-storage-e2e.js <file>
```

All connection details (contract addresses, user keys, SP endpoints) are exported to `devnet-info.json`:

```bash
cat ~/.foc-devnet/state/latest/devnet-info.json
```

See [examples/README.md](examples/README.md) for more usage examples.

---

## ✨ Key Features

### 🪶 Lean Host Requirements
Only needs three things on your machine:
- **tar** archiver
- **rustup/rustc** for building the CLI
- **Docker** for containerized components

Everything else (Lotus, Curio, dependencies) is built inside Docker.

### ⚙️ Configurable Repositories
Depends on 3 repositories, all configurable:
- `filecoin-services` - FOC smart contracts
- `curio` - Next-gen storage provider
- `lotus` - Filecoin daemon

Each can be:
- Auto-downloaded from GitHub (default)
- Linked to your local git repository for development

### 🔒 Deterministic Setup
- **Pinned versions**: All components use specific git tags/commits for reproducibility
- **Deterministic keys**: Uses fixed seeds, generating the same keys on every setup
- **Consistent state**: Each run preserves its context in `~/.foc-devnet/run/<run-id>/step_context.json`

### 🤖 Fully Automated
From building Docker images to deploying contracts—everything is automated:
- Genesis block creation
- Network initialization
- Smart contract deployment
- Storage provider setup

### 🧩 Modular Architecture
Built with modular steps for easy extension and customization:
- Add custom deployment steps
- Configure multiple PDP service providers
- Control "allowed" SP nodes via `~/.foc-devnet/config.toml` (see [Configuration System](README_ADVANCED.md#configuration-system))

### 📜 Programmable
Built for scripting and automation:
- **Contract addresses**: `~/.foc-devnet/run/<run-id>/contract_addresses.json`
- **Step context**: `~/.foc-devnet/run/<run-id>/step_context.json`
- **Latest run symlink**: `~/.foc-devnet/state/latest/` → points to most recent run
- **Custom base directory**: Set `FOC_DEVNET_BASEDIR` env var to override default `~/.foc-devnet` location (see [Environment Variables](README_ADVANCED.md#environment-variables))
- Write scripts for testing, demos, CI/CD pipelines, etc.

### 🌐 Isolated Networks
Uses Docker user-defined networks to mimic real-world node separation—just like production deployments.

### 💰 Built-in Token Contracts
Includes ready-to-use test contracts:
- **MockUSDFC** - ERC-20 test token
- **Multicall3** - Batch contract calls

### 🖥️ Portainer UI
Bundled with Portainer for browser-based Docker management—no terminal wizardry required.

---

## 📋 System Requirements

| Requirement | Details |
|-------------|---------|
| **Rust** | 1.70+ ([rustup.rs](https://rustup.rs)) |
| **Docker** | Desktop (macOS) or CE (Linux) |
| **tar** | Archive utility (usually pre-installed) |
| **Disk Space** | ~20GB for images and blockchain data |
| **Architecture** | Supports both x86 (Intel) and ARM64 (Apple Silicon, AWS Graviton, etc.) architectures. The system automatically selects the appropriate binaries based on your architecture. |

---

## 🛠️ Need More?

See **[README_ADVANCED.md](README_ADVANCED.md)** for comprehensive documentation on:
- **All commands reference** (init, build, start, stop, clean, status, version)
- **Configuration system** (config.toml structure, parameters, editing)
- **Complete directory structure** (what's stored where and why)
- **Resetting and cleanup** (manual cleanup, disk management)
- **Run ID and Step Context** (isolation mechanism, state sharing)
- **Docker and networking** (container architecture, network topology, Portainer debugging)
- **Repository management** (using local repos, sharing configurations)
- **Command flags** (detailed explanations of all flags and when to use them)
- **Lifecycle overview** (full startup sequence, step implementation)
- **Service Provider examples** (1 SP with 0 authorized, 3 SPs with top 2 authorized, etc.)
- **Troubleshooting guides** (port conflicts, build failures, network issues)
- **Additional user actions** (custom genesis, Lotus API access, contract interaction)

---

## 🚶 Examples

See [examples/README.md](examples/README.md).

---

## 📝 License

MIT License - see [LICENSE](LICENSE) file for details.

---

## 💬 Support

- **Issues**: [GitHub Issues](https://github.com/FilOzone/foc-devnet/issues)
