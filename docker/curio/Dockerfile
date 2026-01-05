# Dockerfile for Curio (Ubuntu-based)
#
# This container runs curio, the second generation miner node
# that performs PDP but does not build tipsets.

FROM ubuntu:24.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    bash \
    jq \
    curl \
    postgresql-client \
    libhwloc-dev \
    mesa-utils \
    && rm -rf /var/lib/apt/lists/*

# Create foc-user and foc-group with matching host UID/GID when it is run. The 1002 below is just a placeholder which will be replaced during build process. See `docker.rs`
ARG USER_ID=1002
ARG GROUP_ID=1002
RUN userdel ubuntu || true
RUN groupadd -g ${GROUP_ID} foc-group && \
    useradd -l -u ${USER_ID} -g foc-group -m -s /bin/bash foc-user

# Create necessary directories with proper ownership
RUN mkdir -p /var/tmp/filecoin-proof-parameters \
    && mkdir -p /home/foc-user/.curio \
    && mkdir -p /home/foc-user/curio/fast-storage \
    && mkdir -p /home/foc-user/curio/long-term-storage \
    && chown -R foc-user:foc-group /var/tmp/filecoin-proof-parameters \
    && chown -R foc-user:foc-group /home/foc-user

# Switch to foc-user
USER foc-user

# Copy binaries will be mounted from host
# Binary will be mounted from ~/.foc-localnet/bin

# Expose curio ports
# 12300 - Curio API
# 4701  - GUI
# 4702  - HTTP Subsystem for PDP
# 12310 - HTTP RPC
EXPOSE 12300 4701 4702 12310

# Set environment variables
ENV CURIO_REPO_PATH=/home/foc-user/.curio
ENV FIL_PROOFS_PARAMETER_CACHE=/var/tmp/filecoin-proof-parameters
ENV CGO_CFLAGS_ALLOW="-D__BLST_PORTABLE__"
ENV CGO_CFLAGS="-D__BLST_PORTABLE__"

# Default command will be overridden when container starts
CMD ["/bin/bash"]
