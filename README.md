# RustyLox

<div align="center">

![RustyLox Logo](static/logo.svg)

**Modern Rust rewrite of LoxBerry with Docker containerization**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0) [![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org) [![Rust Edition](https://img.shields.io/badge/Edition-2021-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2021/index.html) [![Language](https://img.shields.io/github/languages/top/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox) [![CI](https://github.com/boernmaster/RustyLox/actions/workflows/ci.yml/badge.svg)](https://github.com/boernmaster/RustyLox/actions/workflows/ci.yml) [![Release](https://github.com/boernmaster/RustyLox/actions/workflows/release.yml/badge.svg)](https://github.com/boernmaster/RustyLox/actions/workflows/release.yml) [![Docker](https://ghcr-badge.egpl.dev/boernmaster/rustylox/latest_tag?trim=major&label=latest)](https://github.com/boernmaster/RustyLox/pkgs/container/rustylox) [![GitHub Stars](https://img.shields.io/github/stars/boernmaster/RustyLox?style=social)](https://github.com/boernmaster/RustyLox/stargazers) [![GitHub Forks](https://img.shields.io/github/forks/boernmaster/RustyLox?style=social)](https://github.com/boernmaster/RustyLox/network/members) [![GitHub Issues](https://img.shields.io/github/issues/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox/issues) [![GitHub Pull Requests](https://img.shields.io/github/issues-pr/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox/pulls) [![Last Commit](https://img.shields.io/github/last-commit/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox/commits/main) [![Contributors](https://img.shields.io/github/contributors/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox/graphs/contributors) [![Docker Image Size](https://img.shields.io/docker/image-size/boernmaster/rustylox/latest)](https://ghcr.io/boernmaster/rustylox) [![Docker Pulls](https://img.shields.io/docker/pulls/boernmaster/rustylox)](https://ghcr.io/boernmaster/rustylox)

</div>

---

## Table of Contents

- [About](#about)
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

- **Original Repository:** [LoxBerry](https://github.com/mschlenstedt/Loxberry)
- **License:** Apache License 2.0 (same as original LoxBerry)
- **Language:** Rust 1.80+ (Edition 2021)
- **Platform:** Docker + Docker Compose
- **Web Framework:** Axum 0.7 + Askama Templates + HTMX
- **MQTT:** rumqttc 0.24 + Eclipse Mosquitto
- **Runtime:** Tokio async runtime

## Architecture

```
loxberry-rust/
├── crates/
│   ├── loxberry-core/       - Common types and errors
│   ├── loxberry-config/     - JSON config management
│   ├── miniserver-client/   - HTTP/UDP Miniserver communication
│   ├── mqtt-gateway/        - MQTT Gateway with transformers
│   ├── plugin-manager/      - Plugin lifecycle management
│   ├── auth/                - JWT authentication & RBAC
│   ├── database/            - PostgreSQL/SQLite abstraction
│   ├── email-manager/       - SMTP email notifications
│   ├── task-scheduler/      - Cron-like task scheduling
│   ├── backup-manager/      - Backup & restore
│   ├── web-api/             - REST API with Axum
│   ├── web-ui/              - Server-rendered web interface (Askama + HTMX)
│   └── loxberry-daemon/     - Main orchestrator binary
├── static/                  - CSS and JavaScript assets
├── volumes/                 - Docker volume mounts
│   ├── config/              - Configuration files
│   ├── data/                - Data storage
│   └── log/                 - Log files
├── sdk/                     - Perl/PHP/Bash SDK compatibility layer
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
- Lifecycle hooks with script execution (preroot, preinstall, postinstall, postroot, uninstall)
- Directory isolation per plugin
- Environment variable injection for full SDK compatibility

### Web UI
- **Dashboard** - System overview and quick links
- **Miniserver Management** - Add, edit, delete, test connections
- **MQTT Monitor** - Real-time message viewer with SSE streaming
- **MQTT Configuration** - Broker settings and authentication
- **Plugin Management** - Install, list, uninstall plugins
- **System Health** - CPU, memory, disk usage with real-time metrics
- **Email Configuration** - SMTP setup and send history
- **Task Scheduler** - Cron-like task management and history
- **Network Diagnostics** - Ping, port scan, connectivity tests
- **Backup & Restore** - Scheduled and manual backups
- **Admin Panel** - User management, API keys, audit log, security settings
- **System Update** - Check GitHub releases and view release notes
- Server-rendered templates (Askama) with HTMX progressive enhancement

### Security
- JWT authentication (HS256) with cookie support
- Role-Based Access Control (RBAC) — Admin, Operator, Viewer, PluginManager
- API key management (`lbx_` prefix, SHA-256 hashing)
- Argon2id password hashing
- Account lockout after 5 failed login attempts
- Security headers middleware (CSP, X-Frame-Options, etc.)
- Comprehensive audit logging

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

#### Authentication
- `POST /api/auth/login` - Login, receive JWT
- `POST /api/auth/logout` - Logout
- `GET /api/users` - List users (admin)
- `POST /api/users` - Create user (admin)

#### System
- `GET /health` - Health check
- `GET /api/system/status` - System status
- `POST /api/backup/create` - Create backup
- `GET /api/backup/list` - List backups

## Quick Start

### Prerequisites
- Docker and Docker Compose

### Option 1: Deploy from pre-built image (recommended)

No build required — pulls the latest image from the GitHub Container Registry.

```bash
# Download the example compose file
curl -fsSL https://raw.githubusercontent.com/boernmaster/RustyLox/main/docker-compose.example.yml \
  -o docker-compose.yml

# Start RustyLox
docker compose up -d

# View logs
docker compose logs -f rustylox
```

**With the built-in Mosquitto broker** (if you don't have an external MQTT broker):

```bash
docker compose --profile mqtt up -d
```

**With an external MQTT broker** — edit `MQTT_BROKER` and `MQTT_PORT` in the compose file before starting:

```yaml
environment:
  - MQTT_BROKER=192.168.1.10   # your broker IP
  - MQTT_PORT=1883
```

### Option 2: Build from source

```bash
git clone https://github.com/boernmaster/RustyLox.git
cd RustyLox

# Create volume directories
mkdir -p volumes/config/system volumes/data/system volumes/log/system

# Build and start
docker compose up -d

# View logs
docker compose logs -f rustylox
```

### Access the Web UI

Open **http://localhost:8080/** in your browser.

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
- **auth**: JWT authentication and RBAC
- **database**: PostgreSQL/SQLite abstraction layer
- **email-manager**: SMTP email notification system
- **task-scheduler**: Cron-like task scheduling
- **backup-manager**: Backup and restore functionality
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
- **Plugins**: Full SDK compatibility layer (Perl/PHP/Bash)
- **MQTT**: Full compatibility with existing MQTT clients

## Roadmap

The core system is production-ready. See [ROADMAP.md](ROADMAP.md) for the full history and future plans.

**Next up**: Advanced features & ecosystem expansion — plugin marketplace, Kubernetes deployment, OAuth2/OIDC, PWA/mobile, plugin sandboxing.

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
- [ROADMAP.md](ROADMAP.md) - Development roadmap

## Support

- **Original LoxBerry**: https://www.loxwiki.eu/display/LOXBERRY/LoxBerry
- **Forum**: https://www.loxforum.com/forum/german/software-konfiguration-und-programmierung/loxberry
- **GitHub Issues**: https://github.com/boernmaster/RustyLox/issues

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
