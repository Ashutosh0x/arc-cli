# arc-cli Docker Image
# Optimized multi-stage build for rapid deployment of the ARC agent framework

# --- Build Stage ---
FROM rust:1.80-slim-bookworm AS builder

# Install required native dependencies (OpenSSL, pkg-config for reqwest/keyring)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy entirely
COPY . .

# Build arc-cli specifically in release mode
RUN cargo build --release -p arc-code

# --- Runtime Stage ---
FROM debian:bookworm-slim

# Ca-certificates are required for outbound HTTPS TLS connections to providers
# Git is required for the git_intel subsystem
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /root/workspace/

# Copy the compiled binary from the builder layer
COPY --from=builder /app/target/release/arc /usr/local/bin/arc

# Default entry to the CLI
ENTRYPOINT ["arc"]
CMD ["--help"]
