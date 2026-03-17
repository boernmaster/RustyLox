# RustyLox

<div align="center">

![RustyLox Logo](static/logo.svg)

**Modern Rust rewrite of LoxBerry with Docker containerization**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![Rust Edition](https://img.shields.io/badge/Edition-2021-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2021/index.html)
[![Language](https://img.shields.io/github/languages/top/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox)

[![CI](https://github.com/boernmaster/RustyLox/actions/workflows/ci.yml/badge.svg)](https://github.com/boernmaster/RustyLox/actions/workflows/ci.yml)
[![Release](https://github.com/boernmaster/RustyLox/actions/workflows/release.yml/badge.svg)](https://github.com/boernmaster/RustyLox/actions/workflows/release.yml)
[![Docker](https://ghcr-badge.egpl.dev/boernmaster/rustylox/latest_tag?trim=major&label=latest)](https://github.com/boernmaster/RustyLox/pkgs/container/rustylox)

[![GitHub Stars](https://img.shields.io/github/stars/boernmaster/RustyLox?style=social)](https://github.com/boernmaster/RustyLox/stargazers)
[![GitHub Forks](https://img.shields.io/github/forks/boernmaster/RustyLox?style=social)](https://github.com/boernmaster/RustyLox/network/members)
[![GitHub Issues](https://img.shields.io/github/issues/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox/issues)
[![GitHub Pull Requests](https://img.shields.io/github/issues-pr/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox/pulls)

[![Last Commit](https://img.shields.io/github/last-commit/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox/commits/main)
[![Contributors](https://img.shields.io/github/contributors/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox/graphs/contributors)
[![Docker Image Size](https://img.shields.io/docker/image-size/boernmaster/rustylox/latest)](https://ghcr.io/boernmaster/rustylox)
[![Docker Pulls](https://img.shields.io/docker/pulls/boernmaster/rustylox)](https://ghcr.io/boernmaster/rustylox)

</div>

---

## 📑 Table of Contents

- [About](#about)
- [Project Status](#project-status)
- [Architecture](#architecture)
- [Features](#features)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Testing](#testing)
- [Docker Services](#docker-services)
- [Development](#development)
- [Differences from Original LoxBerry](#differences-from-original-loxberry)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [Support](#support)
- [Acknowledgments](#acknowledgments)
- [License](#license)

## About

This project is a complete rewrite of [LoxBerry](https://github.com/mschlenstedt/Loxberry) in Rust. LoxBerry is an open-source toolbox for Raspberry Pi that extends the Loxone Smart Home System with additional features like MQTT integration, weather services, and a plugin ecosystem.

- 🏠 **Original Repository:** [LoxBerry](https://github.com/mschlenstedt/Loxberry)
- 📜 **License:** Apache License 2.0 (same as original LoxBerry)
- 🦀 **Language:** Rust 1.80+ (Edition 2021)
- 🐳 **Platform:** Docker + Docker Compose
- 🌐 **Web Framework:** Axum 0.7 + Askama Templates + HTMX
- 📡 **MQTT:** rumqttc 0.24 + Eclipse Mosquitto
- ⚡ **Runtime:** Tokio async runtime

## Project Status

✅ **Phase 1 - Foundation** (Completed)
- Core types and error handling
- Configuration management (JSON)
- Miniserver HTTP/UDP client
- REST API foundation

✅ **Phase 2 - Plugin System** (Completed)
- Plugin manager (install/uninstall/upgrade)
- Plugin database (JSON)
- Lifecycle hooks (preroot, preinstall, postinstall, postroot, uninstall)
- Plugin API endpoints

✅ **Phase 3 - MQTT Gateway** (Completed)
- MQTT broker integration (rumqttc)
- UDP input listener (port 11884)
- Message transformers (built-in + external scripts)
- Bidirectional relay (MQTT ↔ Miniserver)
- Hot-reload for subscriptions and transformers

✅ **Phase 4 - Web UI** (Completed)
- Server-rendered templates (Askama)
- HTMX for dynamic interactions
- Real-time MQTT monitor (Server-Sent Events)
- Miniserver management interface
- Plugin management interface
- System settings page

✅ **Phase 4a - Authentication** (Completed)
- Miniserver credentials (username/password)
- MQTT broker authentication
- Credential storage ready

✅ **Phase 5 - Logging & SDK** (Completed)
- Structured logging framework
- Plugin logging integration
- Backup/restore functionality
- Configuration validation

✅ **Phase 6 - Performance & Monitoring** (Completed)
- Database abstraction layer (PostgreSQL/SQLite)
- Email notification system (SMTP)
- Task scheduler (cron-like)
- Network diagnostics tools
- System health monitoring
- Backup/restore functionality

✅ **Phase 7 - Security Hardening** (Completed)
- JWT authentication & authorization
- Role-Based Access Control (RBAC)
- API key management
- Argon2id password hashing
- Account lockout protection
- Security headers middleware
- Audit logging

## Architecture

```
loxberry-rust/
├── crates/
│   ├── loxberry-core/       - Common types and errors
│   ├── loxberry-config/     - JSON config management
│   ├── miniserver-client/   - HTTP/UDP Miniserver communication
│   ├── mqtt-gateway/        - MQTT Gateway with transformers
│   ├── plugin-manager/      - Plugin lifecycle management
│   ├── web-api/             - REST API with Axum
│   ├── web-ui/              - Server-rendered web interface (Askama + HTMX)
│   └── loxberry-daemon/     - Main orchestrator binary
├── static/                  - CSS and JavaScript assets
├── volumes/                 - Docker volume mounts
│   ├── config/              - Configuration files
│   ├── data/                - Data storage
│   └── log/                 - Log files
├── Dockerfile               - Multi-stage build
└── docker-compose.yml       - Stack with Mosquitto broker
```

## Features

### Miniserver Client
- HTTP/HTTPS communication with Basic Auth
- UDP messaging (max 220 bytes)
- Delta-sending optimization (only send changed values)
- Miniserver reboot detection
- CloudDNS dynamic IP resolution
- SSL certificate verification disabled (for self-signed certs)

### MQTT Gateway
- Connect to Mosquitto broker (or external MQTT broker)
- UDP input listener on port 11884
- Topic subscription management
- Message transformer pipeline:
  - JSON expansion
  - Boolean conversion (true/false/on/off → 1/0)
  - Custom external scripts (Perl/PHP/Bash)
- Bidirectional relay (MQTT ↔ Miniserver)
- Hot-reload for configuration changes

### Plugin System
- Install/uninstall/upgrade plugins from ZIP archives
- Plugin database (JSON at `data/system/plugindatabase.json`)
- MD5-based plugin identity (author + name + folder)
- Lifecycle hooks with script execution
- Directory isolation per plugin
- Environment variable injection for SDK compatibility

### Web UI
- **Dashboard** - System overview and quick links
- **Miniserver Management** - Add, edit, delete, test connections
- **MQTT Monitor** - Real-time message viewer with SSE streaming
- **MQTT Configuration** - Broker settings and authentication
- **Plugin Management** - Install, list, uninstall plugins
- **Settings** - System configuration
- Server-rendered templates (Askama)
- HTMX for progressive enhancement
- Responsive CSS design

### REST API

#### Configuration
- `GET /api/config/general` - Get general.json
- `PUT /api/config/general` - Update configuration

#### Miniserver
- `GET /api/miniserver` - List all Miniservers
- `GET /api/miniserver/:id` - Get Miniserver details
- `POST /api/miniserver/:id/send` - Send HTTP command
- `POST /api/miniserver/:id/get` - Get values
- `POST /api/miniserver/:id/udp` - Send UDP command
- `GET /api/miniserver/:id/status` - Check connection status

#### Plugins
- `GET /api/plugins` - List all installed plugins
- `GET /api/plugins/:md5` - Get plugin details
- `POST /api/plugins/install` - Install plugin from ZIP
- `DELETE /api/plugins/:md5` - Uninstall plugin
- `POST /api/plugins/:md5/upgrade` - Upgrade plugin

#### MQTT Gateway
- `GET /api/mqtt/status` - Gateway status (connected, subscriptions, transformers)
- `POST /api/mqtt/subscriptions/reload` - Hot-reload subscriptions
- `POST /api/mqtt/transformers/reload` - Hot-reload transformers

#### System
- `GET /health` - Health check
- `GET /api/system/status` - System status

## Quick Start

> 📖 **Detailed Build Instructions:** See [BUILD.md](BUILD.md) for comprehensive local build guide

### Prerequisites
- Docker and Docker Compose
- (Optional) Rust 1.80+ for local development

### Option 1: Docker Hub (Recommended)

Pull and run the latest release:

```bash
# Pull the image
docker pull ghcr.io/boernmaster/rustylox:1.0.0

# Create docker-compose.yml
cat > docker-compose.yml << 'EOF'
version: '3.8'
services:
  rustylox:
    image: ghcr.io/boernmaster/rustylox:1.0.0
    container_name: rustylox
    restart: unless-stopped
    ports:
      - "8080:8080"
      - "11884:11884/udp"
    volumes:
      - ./config:/opt/loxberry/config
      - ./data:/opt/loxberry/data
      - ./log:/opt/loxberry/log
    environment:
      - RUST_LOG=info
      - MQTT_BROKER=mosquitto
    depends_on:
      - mosquitto
    networks:
      - loxberry-net

  mosquitto:
    image: eclipse-mosquitto:2.0
    container_name: mosquitto
    restart: unless-stopped
    ports:
      - "1883:1883"
      - "9001:9001"
    volumes:
      - ./mosquitto/config:/mosquitto/config
      - ./mosquitto/data:/mosquitto/data
    networks:
      - loxberry-net

networks:
  loxberry-net:
    driver: bridge
EOF

# Create config directory and start
mkdir -p config/system
docker compose up -d
```

### Option 2: Build from Source

Clone and build locally:

```bash
git clone https://github.com/boernmaster/RustyLox.git
cd RustyLox

# Create volume directories
mkdir -p volumes/config/system volumes/data/system volumes/log/system

# Copy default configuration (or create your own)
cp volumes/config/system/general.json.example volumes/config/system/general.json
# Edit volumes/config/system/general.json with your Miniserver details
```

### 2. Build and Start
```bash
# Build and start all containers
docker compose up -d

# View logs
docker compose logs -f rustylox

# Check status
docker compose ps
```

### 3. Access Web UI
Open your browser to **http://localhost:8080/**

- Dashboard: http://localhost:8080/
- MQTT Monitor: http://localhost:8080/mqtt/monitor
- Miniserver Management: http://localhost:8080/miniserver
- Settings: http://localhost:8080/settings

## Configuration

Configuration is stored in JSON format at `volumes/config/system/general.json`.

Example configuration:
```json
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
      "Useclouddns": "0"
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
```

### MQTT Subscriptions

MQTT subscriptions are configured in `volumes/config/system/mqtt_subscriptions.cfg`:

```ini
[home/#]
[sensors/+/temperature]
```

## Testing

### Web UI
```bash
# Open browser to:
http://localhost:8080/

# Test MQTT monitor real-time streaming:
http://localhost:8080/mqtt/monitor
```

### REST API
```bash
# Health check
curl http://localhost:8080/health

# System status
curl http://localhost:8080/api/system/status

# MQTT Gateway status
curl http://localhost:8080/api/mqtt/status

# Send command to Miniserver
curl -X POST http://localhost:8080/api/miniserver/1/send \
  -H "Content-Type: application/json" \
  -d '{"params": [{"V1": "100"}]}'

# List plugins
curl http://localhost:8080/api/plugins
```

### MQTT Testing
```bash
# Publish message to MQTT broker
docker exec mosquitto mosquitto_pub -t "home/test" -m "Hello from MQTT"

# Subscribe to all messages
docker exec mosquitto mosquitto_sub -t "#" -v

# Send MQTT message via UDP gateway
echo '{"topic":"home/sensor1","value":"25.5"}' | nc -u localhost 11884
```

## Docker Services

The `docker-compose.yml` defines two services:

1. **rustylox** - Main RustyLox application
   - Port 8080: Web UI and REST API
   - Port 11884/udp: MQTT UDP input

2. **mosquitto** - Eclipse Mosquitto MQTT broker
   - Port 1883: MQTT broker
   - Port 9001: WebSocket

## Volume Mounts

When using Docker, the following directories are mounted as volumes:

- `./volumes/config` → `/opt/loxberry/config` (Configuration files)
- `./volumes/data` → `/opt/loxberry/data` (Plugin data, database)
- `./volumes/log` → `/opt/loxberry/log` (Log files)
- `./volumes/plugins` → `/opt/loxberry/plugins` (Plugin files)

## Development

### Local Build
```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run daemon locally
LBHOMEDIR=/tmp/loxberry cargo run --bin loxberry-daemon
```

### Docker Build
```bash
# Rebuild Docker image
docker compose build rustylox

# Restart services
docker compose down && docker compose up -d
```

### Project Structure
Each crate is independent with its own Cargo.toml:

- **loxberry-core**: Common types, errors, results
- **loxberry-config**: JSON config parsing and management
- **miniserver-client**: HTTP/UDP client for Loxone Miniserver
- **mqtt-gateway**: MQTT broker client, UDP listener, transformers
- **plugin-manager**: ZIP extraction, database, lifecycle hooks
- **web-api**: REST API routes and handlers (Axum)
- **web-ui**: Server-rendered templates and UI handlers (Askama)
- **loxberry-daemon**: Main binary that orchestrates all services

## Differences from Original LoxBerry

### Technology Stack
- **Language**: Rust (vs. Perl/PHP/Bash)
- **Runtime**: Single compiled binary (vs. multiple interpreted scripts)
- **Async**: Tokio async runtime (vs. synchronous scripts)
- **Container**: Docker (vs. bare metal Raspberry Pi)

### Architecture
- **Modular**: Separate crates for each component
- **Type-Safe**: Rust's type system prevents many runtime errors
- **Performance**: Compiled code with zero-cost abstractions
- **Modern Web**: Askama templates + HTMX (vs. CGI scripts)

### Compatibility
- **Configuration**: Compatible JSON format with original LoxBerry
- **Plugins**: SDK compatibility layer in development (Phase 5)
- **MQTT**: Full compatibility with existing MQTT clients

## Roadmap

### ✅ Phase 1 - Foundation (Complete)
- Core types and error handling
- Configuration management (JSON)
- Miniserver HTTP/UDP client
- REST API foundation
- Docker containerization

**[📄 View Details](PHASE1_COMPLETE.md)**

### ✅ Phase 2 - Plugin System (Complete)
- Plugin manager (install/uninstall/upgrade)
- Plugin database (JSON)
- Lifecycle hooks (preroot, preinstall, postinstall, postroot, uninstall)
- Plugin API endpoints
- Real plugin testing (Vitoconnect)

**[📄 View Details](PHASE2_COMPLETE.md)**

### ✅ Phase 3 - MQTT Gateway (Complete)
- MQTT broker integration (rumqttc)
- UDP input listener (port 11884)
- Message transformers (built-in + external scripts)
- Bidirectional relay (MQTT ↔ Miniserver)
- Hot-reload for subscriptions and transformers

**[📄 View Details](PHASE3_COMPLETE.md)**

### ✅ Phase 4 - Web UI (Complete)
- Server-rendered templates (Askama)
- HTMX for dynamic interactions
- Real-time MQTT monitor (Server-Sent Events)
- Complete CRUD interfaces
- MQTT configuration with subscriptions/conversions
- RegEx filter expressions
- Professional branding (favicon, logo)

**[📄 View Details](PHASE4_COMPLETE.md)**

### ✅ Phase 5 - SDK Compatibility & Logging (Complete)
- Full SDK compatibility layer (Perl/PHP/Bash)
- Environment variable injection
- Plugin execution wrapper
- Structured logging with rotation
- Log management UI
- Backup & restore functionality

**[📄 View Details](PHASE5_COMPLETE.md)**

### ✅ Phase 6 - Performance & Monitoring (Complete)
- Database abstraction layer (PostgreSQL/SQLite)
- Email notification system (SMTP)
- Task scheduler (cron-like)
- Network diagnostics tools
- System health monitoring
- Backup/restore functionality

**[📄 View Details](PHASE6_COMPLETE.md)**

### ✅ Phase 7 - Security Hardening (Complete)
- JWT authentication & authorization
- Role-Based Access Control (RBAC)
- API key management
- Argon2id password hashing
- Account lockout protection
- Security headers middleware
- Audit logging

**[📄 View Details](PHASE7_COMPLETE.md)**

### 📅 Phase 8 - Advanced Features & Ecosystem (Future)
- High availability & clustering
- Plugin marketplace
- Kubernetes deployment
- OAuth2/OIDC integration
- Two-factor authentication (2FA)
- Mobile app & PWA
- GraphQL API
- Multi-tenancy support
- OpenTelemetry tracing

**[📋 View Plan](PHASE8_PLAN.md)**

## Contributing

We welcome contributions from the community! Whether it's bug reports, feature requests, or code contributions, your input is valuable.

Please read our [Contributing Guide](CONTRIBUTING.md) for details on:
- How to report bugs
- How to suggest features
- Development setup
- Code style guidelines
- Pull request process

Quick start for contributors:
1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes and add tests
4. Run `cargo fmt` and `cargo clippy`
5. Submit a pull request

See also:
- [CHANGELOG.md](CHANGELOG.md) - Version history and changes
- [Phase Documentation](PHASE1_COMPLETE.md) - Detailed phase implementation notes

## Support

- **Original LoxBerry**: https://www.loxwiki.eu/display/LOXBERRY/LoxBerry
- **Forum**: https://www.loxforum.com/forum/german/software-konfiguration-und-programmierung/loxberry
- **GitHub Issues**: <this-repo-issues-url>

## Acknowledgments

Based on the original **LoxBerry** project by Christian Fenzl and the LoxBerry community. Special thanks to all contributors of the original project for creating such a comprehensive smart home platform.

## License

```
Copyright 2024-2026 RustyLox Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

This project maintains the same license as the original [LoxBerry](https://github.com/mschlenstedt/Loxberry) project (Apache License 2.0).
