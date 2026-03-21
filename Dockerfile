# Multi-stage Dockerfile for RustyLox
# Uses cargo-chef for dependency layer caching to speed up builds

# Chef stage - uses prebuilt cargo-chef image (avoids reinstalling on every build)
FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /build

# Planner stage - generates a recipe of dependencies
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage - caches dependencies separately from source
FROM chef AS builder

# Accept build arguments for version info
ARG GIT_HASH=unknown
ARG GIT_TAG=
ARG GIT_DIRTY=false

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

# Set git info as environment variables for build.rs
ENV GIT_HASH=${GIT_HASH}
ENV GIT_TAG=${GIT_TAG}
ENV GIT_DIRTY=${GIT_DIRTY}

RUN cargo build --release --bin loxberry-daemon

# Runtime stage
FROM debian:bookworm-slim

# Metadata labels
LABEL org.opencontainers.image.title="RustyLox"
LABEL org.opencontainers.image.description="Modern Rust rewrite of LoxBerry smart home platform"
LABEL org.opencontainers.image.vendor="RustyLox Contributors"
LABEL org.opencontainers.image.source="https://github.com/boernmaster/RustyLox"
LABEL org.opencontainers.image.licenses="Apache-2.0"

# Install runtime dependencies
# Including Perl, PHP, Bash for plugin SDK compatibility
RUN apt-get update && apt-get install -y \
    ca-certificates \
    perl \
    php-cli \
    php-cgi \
    php-curl \
    php-sqlite3 \
    bash \
    && rm -rf /var/lib/apt/lists/*

# Create loxberry user
RUN useradd -m -u 1000 loxberry

# Create /run/shm as symlink to /dev/shm (plugin compatibility, matches Raspberry Pi)
RUN ln -s /dev/shm /run/shm

# Configure PHP CLI to include LoxBerry SDK libs and auto-prepend bootstrap
# This ensures plugins calling shell_exec("php ...") also find the SDK
RUN PHP_VER=$(php -r 'echo PHP_MAJOR_VERSION.".".PHP_MINOR_VERSION;') && \
    echo "include_path = \".:/opt/loxberry/libs/phplib:/usr/share/php\"" > /etc/php/${PHP_VER}/cli/conf.d/99-loxberry.ini && \
    echo "auto_prepend_file = /opt/loxberry/libs/phplib/loxberry_bootstrap.php" >> /etc/php/${PHP_VER}/cli/conf.d/99-loxberry.ini

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

# Copy system templates for LoxBerry plugin web framework
COPY sdk/templates/system /opt/loxberry/templates/system

# Set permissions
RUN chown -R loxberry:loxberry /opt/loxberry

# Expose ports
EXPOSE 8080

# Set environment variables
ENV LBHOMEDIR=/opt/loxberry
ENV RUST_LOG=info

USER loxberry

ENTRYPOINT ["/usr/local/bin/loxberry-daemon"]
