# foc-devnet

**Run a local Filecoin network with FOC (Filecoin Onchain Cloud) in minutes.**

A developer-friendly tool for spinning up complete Filecoin test networks with smart contract support, deterministic key generation, and automated deployment—all running locally in Docker.

---

## 🚀 Quick Start

### Prerequisites

Use [stable Rust via rustup](https://rustup.rs), Docker, Git, and `tar`.
Default setup also needs network access to download repositories and images.
Run as a non-root user who can run `docker info` without `sudo` (on Linux,
typically through the `docker` group).

**Check `host.docker.internal`**: It must resolve to loopback on the host.
See [hostname setup](docs/networking.md#setup-requirements) before starting.

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

**Re-initializing?** Follow the [stop and clean procedure](docs/cli.md#clean).

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

Use [configuration](docs/configuration.md) to pin or replace dependency sources
and change where devnet data is stored. Try [service provider workflows](docs/service-providers.md)
to test storage with different approval and endorsement settings.

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

- [Advanced documentation](docs/README.md): CLI, configuration, cleanup, and troubleshooting
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
