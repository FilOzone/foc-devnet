# foc-devnet

**Run a local Filecoin network with FOC (Filecoin Onchain Contracts) in minutes.**

A developer-friendly tool for spinning up complete Filecoin test networks with smart contract support, deterministic key generation, and automated deployment—all running locally in Docker.

---

## 🚀 Quick Start

### Prerequisites

Use [stable Rust via rustup](https://rustup.rs), Docker, Git, and `tar`.
Default setup also needs network access to download repositories and images.
Run as a non-root user who can run `docker info` without `sudo` (on Linux,
typically through the `docker` group).

**Check `host.docker.internal`**: It must resolve to `127.0.0.1` on the host.
If it does not, add this entry to `/etc/hosts` once:

```bash
echo '127.0.0.1 host.docker.internal' | sudo tee -a /etc/hosts
```

This is required for SP-to-SP fetch. `foc-devnet start` will check for this and fail with instructions if not configured.

For GitHub Actions, add this step before running foc-devnet:
```yaml
- run: echo '127.0.0.1 host.docker.internal' | sudo tee -a /etc/hosts
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
    --pdp local:/home/user/code/pdp \
    --lotus local:/home/user/code/lotus
```

**Re-initializing?** Run `cargo run -- stop` before deleting devnet state with
`cargo run -- clean`. Config is preserved unless you pass `--all`.

### Step 2: Build

```bash
cargo run -- build lotus
cargo run -- build curio
```

This will:
- Compile Lotus and Curio binaries inside Docker containers
- Cache build artifacts for faster subsequent builds

### Step 3: Start the Network

```bash
cargo run -- start
```

This will:
- Create the genesis block
- Start Lotus daemon with FEVM enabled
- Deploy FOC smart contracts (including MockUSDFC)
- Start storage provider(s)
- Launch [Portainer](https://docs.portainer.io/) for container management

Run only on a trusted development machine and network. Portainer's published
port is not restricted to loopback, and Portainer has access to the Docker socket.

For experimental parallel startup, use `cargo run -- start --parallel`.

### Step 4: Use the Network

After startup, `USER_1` is funded and configured for FOC storage. Check the
cluster with:

```bash
cargo run -- status
```

Client connection details are exported to
`~/.foc-devnet/state/latest/devnet-info.json`. The file contains raw test private
keys: do not share it or use those keys outside the devnet.

See [integration examples](examples/README.md) to check balances or exercise
storage with Synapse or filecoin-pin.

---

## ✨ Key Features

### ⚙️ Configurable Repositories
Depends on these repositories, all configurable:
- `filecoin-services` - FOC smart contracts
- `pdp` - PDP contracts, optionally pinned independently from filecoin-services
- `curio` - Next-gen storage provider
- `lotus` - Filecoin daemon

Each can be:
- Auto-downloaded from GitHub (default)
- Linked to your local git repository for development

### 🔒 Deterministic Setup
- **Pinned defaults**: Default component sources use Git tags or commits
- **Deterministic keys**: Uses fixed seeds, generating the same keys on every setup

### 🤖 Fully Automated
From building Docker images to deploying contracts—everything is automated:
- Genesis block creation
- Network initialization
- Smart contract deployment
- Storage provider setup

### 🧩 Configurable Storage Providers
See [Configuration System](README_ADVANCED.md#configuration-system) for running
multiple PDP service providers with different approval and endorsement settings.

### 📜 Programmable
Use the versioned `devnet-info.json` export for clients and automation.
`FOC_DEVNET_BASEDIR` overrides the default data directory; see
[Environment Variables](README_ADVANCED.md#environment-variables).

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
| **Rust** | Stable via [rustup](https://rustup.rs); a native build toolchain is required |
| **Docker** | Desktop (macOS) or CE (Linux) |
| **tar** | Archive utility (usually pre-installed) |
| **Git** | Required for repository setup |
| **Disk Space** | ~20GB for images and blockchain data |
| **Architecture** | x86-64 and ARM64; the devnet CI workflow runs on Linux x86-64 |

On Debian/Ubuntu, the CLI build uses `build-essential`, `pkg-config`, and
`libssl-dev`.

---

## 🛠️ Need More?

- [Advanced guide](README_ADVANCED.md): CLI, configuration, networking, and troubleshooting
- [Local contributor checks](ci/README.md#local-checks) and [CI dependency profiles](ci/README.md)
- [MockUSDFC test token and deployment guide](contracts/README.md)

---

## 🚶 Examples

See [examples/README.md](examples/README.md).

---

## 📝 License

MIT License - see [LICENSE](LICENSE) file for details.

---

## 💬 Support

- **Issues**: [GitHub Issues](https://github.com/FilOzone/foc-devnet/issues)
