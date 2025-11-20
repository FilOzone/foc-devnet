FROM ubuntu:24.04

# Install system dependencies
RUN apt-get update && apt-get install -y \
    mesa-opencl-icd \
    ocl-icd-opencl-dev \
    gcc \
    git \
    bzr \
    jq \
    pkg-config \
    curl \
    clang \
    build-essential \
    hwloc \
    libhwloc-dev \
    wget \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Go 1.24.7
RUN wget -c https://golang.org/dl/go1.24.7.linux-amd64.tar.gz -O - | tar -xz -C /usr/local
ENV PATH=$PATH:/usr/local/go/bin

# Install Rust (for building proofs if needed)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH=$PATH:/root/.cargo/bin

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/bin/bash"]