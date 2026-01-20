FROM ubuntu:24.04

# Install required dependencies
RUN apt-get update && apt-get install -y \
    wget \
    ca-certificates \
    locales \
    python3 \
    && rm -rf /var/lib/apt/lists/*

# Generate locale to avoid locale-related errors
RUN locale-gen en_US.UTF-8
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

# Create foc-user and foc-group with matching host UID/GID when it is run. The 1002 below is just a placeholder which will be replaced during build process. See `docker.rs`
ARG USER_ID=1002
ARG GROUP_ID=1002
RUN userdel ubuntu || true
RUN GROUP_NAME=$(getent group ${GROUP_ID} | cut -d: -f1 || echo "") && \
    if [ -z "$GROUP_NAME" ]; then \
        groupadd -g ${GROUP_ID} foc-group && GROUP_NAME=foc-group; \
    fi && \
    useradd -l -u ${USER_ID} -g $GROUP_NAME -m -s /bin/bash foc-user

# Set working directory
WORKDIR /yugabyte

# Copy YugabyteDB from build context
# Expects yugabyte directory to be available in the build context
ARG YUGABYTE_SOURCE=yugabyte
COPY ${YUGABYTE_SOURCE} /yugabyte

# Run post-install script and set ownership
RUN GROUP_NAME=$(getent group ${GROUP_ID} | cut -d: -f1) && \
    /yugabyte/bin/post_install.sh && \
    mkdir -p /yugabyte/data && \
    chown -R foc-user:$GROUP_NAME /yugabyte

# Switch to foc-user
USER foc-user

# Expose YugabyteDB ports
# 7000  - YB-Master RPC
# 9000  - YB-Master Admin UI
# 7100  - YB-TServer RPC
# 9100  - YB-TServer Admin UI
# 5433  - YSQL (PostgreSQL-compatible API)
# 9042  - YCQL (Cassandra-compatible API)
# 15433 - YugabyteDB Web UI
EXPOSE 7000 9000 7100 9100 5433 9042 15433
