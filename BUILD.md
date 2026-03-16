# Building RustyLox Locally

This guide covers different ways to build and run RustyLox on your local development machine.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start (Docker)](#quick-start-docker)
- [Native Build (Cargo)](#native-build-cargo)
- [Development Workflow](#development-workflow)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required

- **Docker** 20.10+ and **Docker Compose** 2.0+
  ```bash
  # Check versions
  docker --version
  docker compose version
  ```

### Optional (for native builds)

- **Rust** 1.80+ with cargo
  ```bash
  # Install via rustup
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

  # Check version
  rustc --version
  cargo --version
  ```

- **Git** for cloning the repository
  ```bash
  git --version
  ```

---

## Quick Start (Docker)

**Recommended for most users** - Build and run with Docker.

### 1. Clone the Repository

```bash
git clone https://github.com/boernmaster/RustyLox.git
cd RustyLox
```

### 2. Create Volume Directories

```bash
# Create necessary directories
mkdir -p volumes/config/system
mkdir -p volumes/data/system
mkdir -p volumes/log/system
mkdir -p volumes/plugins
```

### 3. Create Configuration File

Create a minimal configuration file:

```bash
cat > volumes/config/system/general.json << 'EOF'
{
  "Base": {
    "Clouddnsuri": "dns.loxonecloud.com",
    "Lang": "en",
    "Sendstatistic": 1,
    "Startsetup": "1",
    "Systemloglevel": "6",
    "Version": "4.0.0.0"
  },
  "Miniserver": {
    "1": {
      "Name": "My Miniserver",
      "Ipaddress": "192.168.1.100",
      "Port": "80",
      "Admin": "admin",
      "Pass": "password",
      "Useclouddns": "0",
      "Transport": "http",
      "Porthttps": "443"
    }
  },
  "Mqtt": {
    "Brokerhost": "mosquitto",
    "Brokerport": "1883",
    "Brokeruser": "",
    "Brokerpass": "",
    "Udpinport": "11884",
    "Uselocalbroker": "1",
    "Websocketport": "9001"
  },
  "Timeserver": {
    "Timezone": "Europe/Vienna"
  }
}
EOF
```

**Note:** Replace the Miniserver IP, credentials, and timezone with your actual values.

### 4. Build and Start

```bash
# Build the Docker images
docker compose build

# Start all services
docker compose up -d

# View logs
docker compose logs -f loxberry
```

### 5. Access the Web Interface

Open your browser to:
- **Main UI:** http://localhost:8080/
- **Miniserver Monitor:** http://localhost:8080/miniserver/monitor
- **MQTT Monitor:** http://localhost:8080/mqtt/monitor

### 6. Verify Services

```bash
# Check running containers
docker compose ps

# Health check
curl http://localhost:8080/health

# System status
curl http://localhost:8080/api/system/status

# MQTT status
curl http://localhost:8080/api/mqtt/status
```

### 7. Stop Services

```bash
# Stop containers
docker compose down

# Stop and remove volumes (WARNING: deletes all data)
docker compose down -v
```

---

## Native Build (Cargo)

Build and run directly with Rust/Cargo for development.

### 1. Clone the Repository

```bash
git clone https://github.com/boernmaster/RustyLox.git
cd RustyLox
```

### 2. Install Dependencies

```bash
# Update Rust toolchain
rustup update stable

# Install required components
rustup component add rustfmt clippy
```

### 3. Build the Project

```bash
# Build all crates in release mode
cargo build --release

# Or build in debug mode (faster compile, slower runtime)
cargo build

# The binary will be at:
# - Release: target/release/loxberry-daemon
# - Debug: target/debug/loxberry-daemon
```

### 4. Create Local Directories

```bash
# Create LoxBerry directory structure
mkdir -p /tmp/loxberry/{config/system,data/system,log/system,plugins,static}

# Copy static files
cp -r static/* /tmp/loxberry/static/
```

### 5. Create Configuration

```bash
cat > /tmp/loxberry/config/system/general.json << 'EOF'
{
  "Base": {
    "Clouddnsuri": "dns.loxonecloud.com",
    "Lang": "en",
    "Sendstatistic": 1,
    "Startsetup": "1",
    "Systemloglevel": "6",
    "Version": "4.0.0.0"
  },
  "Miniserver": {
    "1": {
      "Name": "My Miniserver",
      "Ipaddress": "192.168.1.100",
      "Port": "80",
      "Admin": "admin",
      "Pass": "password",
      "Useclouddns": "0",
      "Transport": "http",
      "Porthttps": "443"
    }
  },
  "Mqtt": {
    "Brokerhost": "localhost",
    "Brokerport": "1883",
    "Brokeruser": "",
    "Brokerpass": "",
    "Udpinport": "11884",
    "Uselocalbroker": "0",
    "Websocketport": "9001"
  },
  "Timeserver": {
    "Timezone": "Europe/Vienna"
  }
}
EOF
```

### 6. Start MQTT Broker (Optional)

If you want MQTT functionality, run Mosquitto:

```bash
# Using Docker
docker run -d --name mosquitto \
  -p 1883:1883 \
  -p 9001:9001 \
  eclipse-mosquitto:2.0

# Or install locally (Ubuntu/Debian)
sudo apt-get install mosquitto mosquitto-clients
sudo systemctl start mosquitto
```

### 7. Run the Daemon

```bash
# Set environment variables
export LBHOMEDIR=/tmp/loxberry
export RUST_LOG=debug  # or info, warn, error

# Run the daemon
cargo run --release --bin loxberry-daemon

# Or run the compiled binary directly
./target/release/loxberry-daemon
```

### 8. Access the Web Interface

Open http://localhost:8080/ in your browser.

---

## Development Workflow

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p loxberry-core
cargo test -p miniserver-client
cargo test -p mqtt-gateway

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting (don't modify files)
cargo fmt --check

# Run linter
cargo clippy

# Run linter with pedantic warnings
cargo clippy -- -W clippy::pedantic
```

### Build Individual Crates

```bash
# Build specific crate
cargo build -p loxberry-core
cargo build -p miniserver-client
cargo build -p web-ui

# Build with features
cargo build -p mqtt-gateway --features "ssl"
```

### Watch Mode (Auto-rebuild)

Install cargo-watch:

```bash
cargo install cargo-watch
```

Use it for development:

```bash
# Auto-rebuild on changes
cargo watch -x build

# Auto-rebuild and run
cargo watch -x run

# Auto-test on changes
cargo watch -x test
```

### Debugging

```bash
# Build with debug symbols
cargo build

# Run with verbose logging
RUST_LOG=debug cargo run --bin loxberry-daemon

# Use debugger (GDB)
rust-gdb ./target/debug/loxberry-daemon

# Use debugger (LLDB)
rust-lldb ./target/debug/loxberry-daemon
```

### Hot Reload Configuration

While the daemon is running, you can reload configuration:

```bash
# Reload MQTT subscriptions
curl -X POST http://localhost:8080/api/mqtt/subscriptions/reload

# Reload MQTT transformers
curl -X POST http://localhost:8080/api/mqtt/transformers/reload

# Restart the daemon to reload general.json
```

---

## Troubleshooting

### Docker Build Issues

**Problem:** Permission denied on Cargo.lock

```bash
# Fix ownership
sudo chown -R $USER:$USER .
```

**Problem:** Docker daemon not running

```bash
# Start Docker (Linux)
sudo systemctl start docker

# Check status
sudo systemctl status docker
```

**Problem:** Port already in use

```bash
# Check what's using port 8080
sudo lsof -i :8080

# Kill the process or change port in docker-compose.yml
```

### Rust Build Issues

**Problem:** rustc version too old

```bash
# Update Rust
rustup update stable
rustup default stable
```

**Problem:** Linking errors

```bash
# Install build dependencies (Ubuntu/Debian)
sudo apt-get install build-essential pkg-config libssl-dev

# Install build dependencies (macOS)
xcode-select --install
brew install openssl
```

**Problem:** Out of memory during build

```bash
# Reduce parallel jobs
cargo build -j 2

# Or disable LTO in Cargo.toml temporarily
```

### Runtime Issues

**Problem:** Cannot connect to Miniserver

1. Check Miniserver IP in `general.json`
2. Verify credentials
3. Test connection:
   ```bash
   curl http://localhost:8080/api/miniserver/1/status
   ```

**Problem:** MQTT broker connection failed

1. Verify Mosquitto is running:
   ```bash
   docker compose ps mosquitto
   # or
   sudo systemctl status mosquitto
   ```

2. Check MQTT settings in `general.json`
3. Test connection:
   ```bash
   mosquitto_pub -t test -m "hello"
   ```

**Problem:** Web UI shows 404 errors

1. Verify static files are copied:
   ```bash
   ls $LBHOMEDIR/static/
   ```

2. Check logs:
   ```bash
   docker compose logs loxberry
   # or
   tail -f /tmp/loxberry/log/system/loxberry.log
   ```

### Getting Help

If you encounter issues:

1. Check existing issues: https://github.com/boernmaster/RustyLox/issues
2. Review logs: `docker compose logs loxberry`
3. Enable debug logging: `RUST_LOG=debug`
4. Open a new issue with:
   - Rust version (`rustc --version`)
   - Docker version (`docker --version`)
   - OS and architecture
   - Full error output

---

## Next Steps

- Read [CLAUDE.md](CLAUDE.md) for development guidelines
- See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guide
- Check [README.md](README.md) for feature documentation
- Review [CHANGELOG.md](CHANGELOG.md) for version history

## Build Information

- **Minimum Rust Version:** 1.80
- **Rust Edition:** 2021
- **Build Time (Release):** ~2-5 minutes (depending on hardware)
- **Binary Size (Release):** ~15-20 MB
- **Docker Image Size:** ~100 MB

---

**Happy Building! 🦀**
