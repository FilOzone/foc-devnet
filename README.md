# foc-localnet

A command-line tool for managing local Filecoin-onchain-cloud clusters.

## Quick Start

```bash
# Install foc-localnet
cargo install --git https://github.com/FilOzone/foc-localnet.git

# Install shell completions (optional)
foc-localnet completions --install

# Check version and build info
foc-localnet version

# Check system requirements and install dependencies
foc-localnet requirements --setup

# Initialize the environment (build Docker images)
foc-localnet init

# Start the local cluster
foc-localnet start

# Check status
foc-localnet status

# Stop the cluster
foc-localnet stop
```

## Installation

### From Crates.io (when published)

```bash
cargo install foc-localnet
```

### From Source

```bash
git clone https://github.com/FilOzone/foc-localnet.git
cd foc-localnet
cargo install --path .
```

The binary will be installed to `~/.cargo/bin/foc-localnet` (which should be in your PATH).

### System-wide Installation

To install to `/opt/bin` or another system directory:

```bash
# Build the binary
cargo build --release

# Install to /opt/bin (requires sudo)
sudo cp target/release/foc-localnet /opt/bin/

# Or install via cargo with custom root
cargo install --path . --root /opt
```

### Shell Completion

Install shell completion scripts automatically:

```bash
# Auto-detect shell and install to appropriate location
foc-localnet completions --install

# Install for specific shell
foc-localnet completions bash --install
foc-localnet completions zsh --install
foc-localnet completions fish --install
```

Or generate scripts manually:

```bash
# Auto-detect shell and output to stdout
foc-localnet completions > completion_script

# Generate for specific shell
foc-localnet completions bash > ~/.bash_completion.d/foc-localnet
foc-localnet completions zsh > ~/.zsh/completions/_foc-localnet
foc-localnet completions fish > ~/.config/fish/completions/foc-localnet.fish
```

The `--install` flag automatically chooses the best location:
- **System-wide** (if writable): `/etc/bash_completion.d/`, `/usr/local/share/zsh/site-functions/`
- **User-specific** (fallback): `~/.bash_completion.d/`, `~/.zsh/completions/`, `~/.config/fish/completions/`

## Commands

### Core Commands

- `foc-localnet start` - Start the local Filecoin cluster
- `foc-localnet stop` - Stop the running cluster
- `foc-localnet status` - Show cluster status and system information
- `foc-localnet version` - Show version, commit ID, and build information
- `foc-localnet requirements` - Check system requirements
- `foc-localnet requirements --setup` - Check and automatically install missing dependencies

### Initialization & Building

- `foc-localnet init` - Initialize environment and build Docker images
- `foc-localnet build lotus` - Build Lotus binaries
- `foc-localnet build curio` - Build Curio binaries

### Configuration

- `foc-localnet config lotus <source>` - Configure Lotus source location
- `foc-localnet config curio <source>` - Configure Curio source location

### Maintenance

- `foc-localnet clean` - Clean all artifacts, binaries, and Docker images
- `foc-localnet clean --artifacts` - Clean only downloaded artifacts
- `foc-localnet clean --dockerimages` - Clean only Docker images
- `foc-localnet clean --binaries` - Clean only built binaries

## Overview

`foc-localnet` provides an easy way to start, stop, and manage local Filecoin network clusters for development and testing purposes. It checks system requirements and can install some dependencies automatically.

## Features

- 🚀 **Start/Stop Clusters**: Easily manage local Filecoin clusters
- 🔍 **Requirements Checking**: Automatically verify system prerequisites
- 📦 **Auto-Installation**: Install missing dependencies (Homebrew) with `--setup`
- 🏗️ **Build Projects**: Build Lotus and Curio from source in Docker containers
- 🧹 **Clean Environment**: Clean artifacts, binaries, and Docker images
- 🖥️ **Cross-Platform**: Supports macOS and Ubuntu/Debian Linux
- 🎨 **Beautiful Output**: Colorized terminal output with emojis

## Prerequisites

- Rust 1.70+ (install via rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Docker
- Homebrew (macOS only, will be installed automatically with `--setup`)

## FOC Contract Deployment

When you start the cluster with `foc-localnet start`, the following happens automatically:

1. **Lotus** starts with FEVM enabled for Ethereum RPC support
2. **Lotus-Miner** builds blocks and maintains the chain
3. **FOC Contracts** are deployed to the local network:
   - MockUSDFC token (toy ERC-20 for testing)
   - PDP Verifier contracts
   - Warm Storage Service contracts
   - Service Provider Registry
4. **Yugabyte** database starts for Curio
5. **Curio** second-generation miner connects to FOC contracts

### Contract Addresses

After deployment, all contract addresses are saved to:
```
~/.foc-localnet/artifacts/docker/volumes/foc-contract-addresses.json
```

### Fund Transfer Chain

```
GLOBAL_FIL_FAUCET (50,000 FIL from genesis)
    ↓ 10,000 FIL
FEVM_FAUCET (f4 address for FEVM operations)
    ↓ 5,000 FIL  
FOC_DEPLOYER (f4 address that deploys contracts)
```

For detailed information about FOC deployment, see [docs/FOC_DEPLOYMENT.md](docs/FOC_DEPLOYMENT.md).

### Get Help

```bash
foc-localnet --help
foc-localnet <command> --help
```

## System Requirements

### macOS
- Homebrew (auto-installed with `--setup`)
- Docker Desktop

### Ubuntu/Debian Linux
- Docker CE
- sudo access for package installation

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Code Quality

```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
```

## CI/CD

This project uses GitHub Actions for continuous integration:

- **Build & Test**: Runs on Ubuntu latest
- **Requirements Check**: Validates setup functionality on macOS and Ubuntu 24.04
- **Code Quality**: Enforces formatting and linting standards

## Project Structure

```
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library exports
├── app.rs                  # Application initialization
├── cli.rs                  # CLI argument parsing
├── config.rs               # Configuration management
└── commands/
    ├── mod.rs              # Command exports
    ├── start.rs            # Cluster start logic
    ├── stop.rs            # Cluster stop logic
    ├── clean.rs           # Environment cleaning logic
    ├── build/
    │   ├── mod.rs         # Build command logic
    │   └── repository.rs  # Repository preparation logic
    └── requirements/
        ├── mod.rs         # Requirements checking logic
        └── setup_docker/
            ├── mod.rs     # Docker setup dispatcher
            ├── macos.rs   # macOS-specific setup
            └── linux.rs   # Linux-specific setup
```

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes and add tests
4. Run the test suite: `cargo test`
5. Ensure code quality: `cargo fmt && cargo clippy`
6. Commit your changes: `git commit -am 'Add your feature'`
7. Push to the branch: `git push origin feature/your-feature`
8. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

For questions, issues, or contributions, please:

- Open an issue on [GitHub](https://github.com/FilOzone/foc-localnet/issues)
- Check existing documentation and examples
- Review the code for implementation details
