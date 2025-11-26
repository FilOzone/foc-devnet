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

# Create foc-user and foc-group with matching host UID/GID when it is run. The 1002 below is just a placeholder which will be replaced during build process. See `docker.rs`
ARG USER_ID=1002
ARG GROUP_ID=1002
RUN groupadd -g ${GROUP_ID} foc-group && \
    useradd -l -u ${USER_ID} -g foc-group -m -s /bin/bash foc-user && \
    mkdir -p /home/foc-user/go/pkg && \
    chown -R foc-user:foc-group /home/foc-user/go

# Install Rust as foc-user
USER foc-user
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH=$PATH:/home/foc-user/.cargo/bin

# Set working directory
WORKDIR /workspace

# Define volumes for external access
VOLUME ["/home/foc-user/.cargo", "/home/foc-user/.rustup", "/home/foc-user/go", "/var/tmp/filecoin-proof-parameters"]

# Default command
CMD ["/bin/bash"]