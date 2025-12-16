# Dockerfile for Lotus Miner (Ubuntu-based)
#
# This container runs the lotus-miner, the first generation miner node
# that builds tipsets and performs PoRep (original Filecoin).

FROM ubuntu:24.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    bash \
    jq \
    curl \
    libhwloc-dev \
    mesa-utils \
    && rm -rf /var/lib/apt/lists/*

# Create foc-user and foc-group with matching host UID/GID when it is run. The 1002 below is just a placeholder which will be replaced during build process. See `docker.rs`
ARG USER_ID=1002
ARG GROUP_ID=1002
RUN groupadd -g ${GROUP_ID} foc-group && \
    useradd -l -u ${USER_ID} -g foc-group -m -s /bin/bash foc-user

# Create necessary directories with proper ownership
RUN mkdir -p /var/tmp/filecoin-proof-parameters \
    && mkdir -p /home/foc-user/.lotus-miner-local-net \
    && mkdir -p /home/foc-user/.lotus-local-net \
    && mkdir -p /home/foc-user/.genesis-sectors \
    && chown -R foc-user:foc-group /var/tmp/filecoin-proof-parameters \
    && chown -R foc-user:foc-group /home/foc-user

# Switch to foc-user
USER foc-user

# Copy binaries will be mounted from host
# Binary will be mounted from ~/.foc-localnet/bin

# Expose lotus-miner ports
# 2345 - Lotus Miner API
# 2346 - Lotus Miner P2P
EXPOSE 2345 2346

# Set environment variables for local devnet
ENV LOTUS_MINER_PATH=/home/foc-user/.lotus-miner-local-net
ENV LOTUS_PATH=/home/foc-user/.lotus-local-net
ENV LOTUS_SKIP_GENESIS_CHECK=_yes_
ENV FIL_PROOFS_PARAMETER_CACHE=/var/tmp/filecoin-proof-parameters
ENV CGO_CFLAGS_ALLOW="-D__BLST_PORTABLE__"
ENV CGO_CFLAGS="-D__BLST_PORTABLE__"

# Default command will be overridden when container starts
CMD ["/bin/bash"]
