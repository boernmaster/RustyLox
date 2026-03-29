# Multi-stage Dockerfile for RustyLox
# Uses cargo-chef for dependency layer caching to speed up builds

# Chef stage - uses prebuilt cargo-chef image (avoids reinstalling on every build)
FROM lukemathwalker/cargo-chef:latest-rust-bullseye AS chef
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

RUN cargo build --release --bin rustylox-daemon

# Runtime stage - uses Debian 11 (bullseye) for PHP 7.4 compatibility
# PHP 8.x removed curly-brace string offset syntax used by older plugins
FROM debian:bullseye-slim

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
    libconfig-simple-perl \
    liburi-perl \
    libhtml-template-perl \
    libjson-perl \
    libcgi-pm-perl \
    libwww-perl \
    libdbi-perl \
    libdbd-sqlite3-perl \
    cpanminus \
    php-cli \
    php-cgi \
    php-curl \
    php-sqlite3 \
    bash \
    dnsmasq \
    procps \
    sudo \
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
COPY --from=builder /build/target/release/rustylox-daemon /usr/local/bin/

# Copy static files (CSS, JS)
COPY static /opt/loxberry/static

# Copy SDK libraries for plugin compatibility
COPY sdk/perllib /opt/loxberry/libs/perllib
COPY sdk/phplib /opt/loxberry/libs/phplib
COPY sdk/bashlib /opt/loxberry/libs/bashlib

# Copy system templates for LoxBerry plugin web framework
COPY sdk/templates/system /opt/loxberry/templates/system

# Set permissions
# Install Perl modules not available in Debian bullseye repos
RUN cpanm --notest List::MoreUtils 2>/dev/null

RUN chown -R loxberry:loxberry /opt/loxberry

# ── dnsmasq setup (done at build time as root) ────────────────────────────────
# Create the drop-in config dir and make it writable by the loxberry group so
# the running daemon can write weather redirect configs without root.
RUN mkdir -p /etc/dnsmasq.d \
    && chown root:loxberry /etc/dnsmasq.d \
    && chmod 775 /etc/dnsmasq.d

# Allow loxberry to send SIGHUP to dnsmasq without a password via sudo
RUN echo "loxberry ALL=(root) NOPASSWD: /usr/bin/pkill -HUP dnsmasq" \
        > /etc/sudoers.d/loxberry-dnsmasq \
    && chmod 440 /etc/sudoers.d/loxberry-dnsmasq

# Write the base dnsmasq config: forward everything except the weather override
RUN printf '# RustyLox dnsmasq – DNS redirect for Loxone Cloud Emulator\n# Listen on port 5353 (unprivileged); docker-compose maps host:53 -> container:5353\nport=5353\nno-resolv\nserver=8.8.8.8\nserver=8.8.4.4\nconf-dir=/etc/dnsmasq.d/,*.conf\n' \
        > /etc/dnsmasq.conf

# Copy entrypoint script (runs as root, sets up dnsmasq, then drops to loxberry)
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Expose ports
EXPOSE 8080
EXPOSE 6066
EXPOSE 5353/udp

# Set environment variables
ENV LBHOMEDIR=/opt/loxberry
ENV RUST_LOG=info

# Note: USER is not set here; docker-entrypoint.sh drops to loxberry after root setup

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
