# Multi-stage Dockerfile for LoxBerry Rust

# Build stage
FROM rust:bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml ./
COPY crates ./crates

# Build release binary
RUN cargo build --release --bin loxberry-daemon

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
# Including Perl, PHP, Bash for future plugin compatibility
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create loxberry user
RUN useradd -m -u 1000 loxberry

# Set up directory structure
WORKDIR /opt/loxberry

RUN mkdir -p \
    /opt/loxberry/bin \
    /opt/loxberry/config/system \
    /opt/loxberry/data/system \
    /opt/loxberry/log/system \
    /opt/loxberry/log/system_tmpfs \
    /opt/loxberry/webfrontend/htmlauth \
    /opt/loxberry/webfrontend/html \
    /opt/loxberry/templates/system

# Copy binary from builder
COPY --from=builder /build/target/release/loxberry-daemon /usr/local/bin/

# Set permissions
RUN chown -R loxberry:loxberry /opt/loxberry

# Expose ports
EXPOSE 8080

# Set environment variables
ENV LBHOMEDIR=/opt/loxberry
ENV RUST_LOG=info

USER loxberry

ENTRYPOINT ["/usr/local/bin/loxberry-daemon"]
