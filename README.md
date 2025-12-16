# foc-localnet

A command-line tool for running local Filecoin networks with FOC (Filecoin Onchain Contracts) for development and testing.

## Quick Start

```bash
# 1. Install dependencies and check requirements
foc-localnet requirements --setup

# 2. Initialize (builds Docker images and prepares environment)
foc-localnet init

# 3. Start the local Filecoin cluster
foc-localnet start

# 4. Stop the cluster when done
foc-localnet stop
```

That's it! Your local Filecoin network is now running with FOC contracts deployed.

## What Gets Started

When you run `foc-localnet start`, the following components are automatically deployed:

1. **Lotus** - Filecoin daemon with FEVM (Filecoin EVM) enabled
2. **Lotus-Miner** - Block producer for the local network
3. **FOC Contracts** - Smart contracts including:
   - MockUSDFC token (test ERC-20)
   - PDP Verifier
   - Warm Storage Service
   - Service Provider Registry
4. **YugabyteDB** - Database for Curio miner
5. **Curio** - Second-generation storage provider (optional)

Contract addresses are saved to: `~/.foc-localnet/artifacts/docker/volumes/foc-contract-addresses.json`

## Installation

### From Source

```bash
git clone https://github.com/FilOzone/foc-localnet.git
cd foc-localnet
cargo install --path .
```

The binary will be installed to `~/.cargo/bin/foc-localnet`.

### Shell Completions (Optional)

```bash
# Auto-detect your shell and install completions
foc-localnet completions --install
```

## Essential Commands

| Command | Description |
|---------|-------------|
| `foc-localnet init` | Build Docker images and prepare environment (run once) |
| `foc-localnet start` | Start the local Filecoin cluster |
| `foc-localnet stop` | Stop all running containers |
| `foc-localnet status` | View cluster status and system information |
| `foc-localnet clean` | Remove all artifacts and reset environment |

## System Requirements

- **Rust** 1.70+ (install from [rustup.rs](https://rustup.rs))
- **Docker** Desktop (macOS) or Docker CE (Linux)
- **Disk Space** ~20GB for Docker images and blockchain data
- **macOS** - Homebrew (auto-installed with `--setup`)
- **Linux** - Ubuntu/Debian (apt-based)

## Common Use Cases

### Reset Everything and Start Fresh

```bash
foc-localnet clean         # Remove all data
foc-localnet init          # Rebuild images
foc-localnet start         # Start fresh cluster
```

### Check Cluster Status

```bash
foc-localnet status
```

This shows:
- Running containers and uptime
- Disk usage
- Build versions
- Git repository status

### Advanced Configuration

```bash
# Use local Lotus source for development
foc-localnet config lotus local --dir /path/to/lotus

# Use specific Git branch
foc-localnet config lotus git --repo https://github.com/filecoin-project/lotus --branch master

# Rebuild specific component
foc-localnet build lotus
foc-localnet build curio
```

## Troubleshooting

### Port Already in Use

If you see port conflicts:
```bash
foc-localnet stop          # Stop all containers
lsof -i :1234              # Check what's using the port
```

### Cluster Won't Start

```bash
foc-localnet clean         # Clean everything
foc-localnet init          # Reinitialize
foc-localnet start         # Try again
```

### Docker Permission Issues

On Linux, ensure your user is in the `docker` group:
```bash
sudo usermod -aG docker $USER
newgrp docker
```

## Data Locations

All data is stored in `~/.foc-localnet/`:

```
~/.foc-localnet/
├── artifacts/          # Built binaries and Docker images
├── logs/              # Container logs
├── repos/             # Cloned Git repositories
├── state/             # Runtime state
└── config.toml        # Configuration file
```

## Getting Help

```bash
foc-localnet --help                    # General help
foc-localnet <command> --help          # Command-specific help
```

## Contributing

We welcome contributions! Please:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes and run tests: `cargo test`
4. Ensure code quality: `cargo fmt && cargo clippy`
5. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/FilOzone/foc-localnet/issues)
- **Documentation**: See `.github/copilot-instructions.md` for detailed architecture
- **Community**: Join the FilOzone community discussions
