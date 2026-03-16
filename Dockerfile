# Multi-stage Dockerfile for LoxBerry Rust
# Uses cargo-chef for dependency layer caching to speed up builds

# Chef stage - installs cargo-chef
FROM rust:bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /build

# Planner stage - generates a recipe of dependencies
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage - caches dependencies separately from source
FROM chef AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Build dependencies only (cached layer as long as Cargo.toml/Cargo.lock don't change)
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build the actual application
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --bin loxberry-daemon

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
# Including Perl, PHP, Bash for plugin SDK compatibility
RUN apt-get update && apt-get install -y \
    ca-certificates \
    perl \
    php-cli \
    bash \
    && rm -rf /var/lib/apt/lists/*

# Create loxberry user
RUN useradd -m -u 1000 loxberry

# Set up directory structure
WORKDIR /opt/loxberry

RUN mkdir -p \
    /opt/loxberry/bin \
    /opt/loxberry/sbin \
    /opt/loxberry/config/system \
    /opt/loxberry/config/plugins \
    /opt/loxberry/data/system \
    /opt/loxberry/data/plugins \
    /opt/loxberry/data/backup \
    /opt/loxberry/log/system \
    /opt/loxberry/log/system_tmpfs \
    /opt/loxberry/log/plugins \
    /opt/loxberry/webfrontend/htmlauth/system \
    /opt/loxberry/webfrontend/htmlauth/plugins \
    /opt/loxberry/webfrontend/html/system \
    /opt/loxberry/webfrontend/html/plugins \
    /opt/loxberry/templates/system \
    /opt/loxberry/templates/plugins \
    /opt/loxberry/libs/perllib \
    /opt/loxberry/libs/phplib \
    /opt/loxberry/libs/bashlib

# Copy binary from builder
COPY --from=builder /build/target/release/loxberry-daemon /usr/local/bin/

# Copy static files (CSS, JS)
COPY static /opt/loxberry/static

# Copy SDK libraries for plugin compatibility
COPY sdk/perllib /opt/loxberry/libs/perllib
COPY sdk/phplib /opt/loxberry/libs/phplib
COPY sdk/bashlib /opt/loxberry/libs/bashlib

# Set permissions
RUN chown -R loxberry:loxberry /opt/loxberry

# Expose ports
EXPOSE 8080

# Set environment variables
ENV LBHOMEDIR=/opt/loxberry
ENV RUST_LOG=info

USER loxberry

ENTRYPOINT ["/usr/local/bin/loxberry-daemon"]
