# foc-localnet

A command-line tool for managing local Filecoin-onchain-cloud clusters.

## Overview

`foc-localnet` provides an easy way to start, stop, and manage local Filecoin network clusters for development and testing purposes. It handles system requirements automatically and provides a streamlined experience for developers working with Filecoin-onchain-cloud.

## Features

- 🚀 **Start/Stop Clusters**: Easily manage local Filecoin clusters
- 🔍 **Requirements Checking**: Automatically verify system prerequisites
- 📦 **Auto-Installation**: Install missing dependencies (Docker, Homebrew) with `--setup`
- 🖥️ **Cross-Platform**: Supports macOS and Ubuntu/Debian Linux
- 🎨 **Beautiful Output**: Colorized terminal output with emojis

## Installation

### Prerequisites

- Rust 1.70+ (with rustup)
- Docker (will be installed automatically with `--setup`)
- Homebrew (macOS only, will be installed automatically with `--setup`)

### Build from Source

```bash
git clone https://github.com/FilOzone/foc-localnet.git
cd foc-localnet
cargo build --release
```

The binary will be available at `target/release/foc-localnet`.

## Usage

### Check System Requirements

```bash
# Check if all requirements are met
foc-localnet requirements-checker

# Check and automatically install missing dependencies
foc-localnet requirements-checker --setup
```

### Manage Clusters

```bash
# Start the local cluster
foc-localnet start

# Stop the local cluster
foc-localnet stop
```

### Get Help

```bash
foc-localnet --help
foc-localnet requirements-checker --help
```

## System Requirements

### macOS
- Homebrew (auto-installed with `--setup`)
- Docker Desktop (auto-installed with `--setup`)

### Ubuntu/Debian Linux
- Docker CE (auto-installed with `--setup`)
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
    ├── stop.rs             # Cluster stop logic
    └── requirements_checker/
        ├── mod.rs          # Requirements checking logic
        └── setup_docker/
            ├── mod.rs      # Docker setup dispatcher
            ├── macos.rs    # macOS-specific setup
            └── linux.rs    # Linux-specific setup
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
