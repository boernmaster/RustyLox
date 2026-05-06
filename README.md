# RustyLox

<div align="center">

![RustyLox Logo](static/logo.svg)

**LoxBerry as a Docker container — the open-source smart home bridge for Loxone, rewritten in Rust**

[![CI](https://github.com/boernmaster/RustyLox/actions/workflows/ci.yml/badge.svg)](https://github.com/boernmaster/RustyLox/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/boernmaster/RustyLox)](https://github.com/boernmaster/RustyLox/releases/latest)
[![Docker](https://img.shields.io/docker/v/boernmaster/rustylox?registry_uri=https://ghcr.io&label=docker)](https://github.com/boernmaster/RustyLox/pkgs/container/rustylox)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org)

</div>

---

RustyLox is [LoxBerry](https://github.com/mschlenstedt/Loxberry) as a Docker container. The goal is to provide the same set of functions — MQTT gateway, plugin system, Miniserver communication, web management interface — without requiring a dedicated Raspberry Pi or a bare-metal OS install. You run one `docker compose up` and everything is there.

The core platform is production-ready. **Plugin ecosystem compatibility is a work in progress** — the infrastructure (ZIP install, lifecycle hooks, Perl/PHP/Bash SDK, directory layout) is in place, but not every existing LoxBerry plugin is tested or guaranteed to work. Contributions and test reports are welcome.

## Quick Start

**Requires:** Docker and Docker Compose

```bash
# Download the example compose file
curl -fsSL https://raw.githubusercontent.com/boernmaster/RustyLox/main/docker-compose.example.yml \
  -o docker-compose.yml

# Download the env template and fill in your secrets (see Setup below)
curl -fsSL https://raw.githubusercontent.com/boernmaster/RustyLox/main/.env.example -o .env
```

Edit `.env`, then start:

```bash
# With the built-in Mosquitto broker
docker compose --profile mqtt up -d

# Or without, if you already run your own broker
docker compose up -d
```

Open **http://localhost:8080** and log in with the `admin` account using the password you set in `ADMIN_PASSWORD`.

## Setup

RustyLox requires two environment variables before it will start. They are read from a `.env` file placed next to `docker-compose.yml`.

### 1. Create `.env`

```bash
cp .env.example .env
```

### 2. Generate a JWT secret

```bash
openssl rand -hex 32
```

Paste the output as `JWT_SECRET` in `.env`.

### 3. Set the admin password

Set `ADMIN_PASSWORD` to a password of at least 8 characters. RustyLox creates the `admin` account from this value on first boot. Once the account exists you can remove the variable — it is no longer read.

### Example `.env`

```dotenv
JWT_SECRET=fb97926ea253a7d988ce0544733445fff78d2a35039b9a582c4568e08ec5ed6f
ADMIN_PASSWORD=changeme123
```

> **Never commit `.env` to version control.** It is already listed in `.gitignore`.

### Environment variable reference

| Variable | Required | Description |
|----------|----------|-------------|
| `JWT_SECRET` | Always | Signs authentication tokens. Minimum 32 characters. Rotate by replacing the value and restarting — all active sessions are invalidated. |
| `ADMIN_PASSWORD` | First boot only | Creates the initial `admin` user. Minimum 8 characters. Remove after first start. |
| `MQTT_BROKER` | No | Hostname of the MQTT broker (default: `mosquitto`). |
| `MQTT_PORT` | No | MQTT broker port (default: `1883`). |
| `BIND_ADDR` | No | Address and port the HTTP server listens on (default: `0.0.0.0:8080`). |
| `LBHOMEDIR` | No | Base directory for config, data, and log volumes (default: `/opt/loxberry`). |
| `RUST_LOG` | No | Log level: `error`, `warn`, `info`, `debug`, `trace` (default: `info`). |

> Build from source — see [docs/development.md](docs/development.md).

## What It Does

| Feature | Description |
|---------|-------------|
| **MQTT Gateway** | Bridges MQTT topics to Loxone Virtual Inputs; supports transformers, hot-reload, UDP input |
| **Miniserver Client** | HTTP/UDP communication, delta-sending, CloudDNS, reboot detection |
| **Plugin System** | Install/uninstall plugins from ZIP archives; Perl/PHP/Bash SDK compatibility layer (work in progress — not all plugins work yet) |
| **Web UI** | Dashboard, MQTT monitor, plugin management, admin panel, backup, task scheduler |
| **REST API** | Full API at `/api/*`; interactive docs at `/api-docs` |
| **Security** | JWT auth, RBAC (Admin/Operator/Viewer), API keys, Argon2id, audit log |

## Documentation

| Document | Contents |
|----------|----------|
| [docs/architecture.md](docs/architecture.md) | Crate layout, tech stack, differences from original LoxBerry |
| [docs/configuration.md](docs/configuration.md) | `general.json` reference, environment variables, ports, volume mounts |
| [docs/api.md](docs/api.md) | Full REST API reference |
| [docs/plugins.md](docs/plugins.md) | Plugin development: structure, hooks, SDK, environment variables |
| [docs/development.md](docs/development.md) | Build from source, testing, debugging, local workflow |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to report bugs, submit PRs, commit style |
| [CHANGELOG.md](CHANGELOG.md) | Version history |
| [ROADMAP.md](ROADMAP.md) | Planned features |

## Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| `8080` | TCP | Web UI and REST API (example compose) |
| `1883` | TCP | MQTT broker (Mosquitto) |
| `6066` | TCP | Loxone Cloud Emulator (weather) |
| `53` | UDP/TCP | DNS redirect (dnsmasq) |
| `8090` | UDP | Miniserver Virtual UDP Output |
| `11884` | UDP | MQTT UDP input gateway |

## Credits

RustyLox is built on the shoulders of the original **[LoxBerry](https://github.com/mschlenstedt/Loxberry)** project, created by **Christian Fenzl** and the LoxBerry community. Their work established the plugin ecosystem, configuration format, and SDK that RustyLox remains compatible with.

### Third-Party Components

| Component | Author / Maintainer | License |
|-----------|--------------------|---------| 
| [LoxBerry](https://github.com/mschlenstedt/Loxberry) | Christian Fenzl & contributors | Apache 2.0 |
| [Axum](https://github.com/tokio-rs/axum) | Tokio project | MIT |
| [Tokio](https://github.com/tokio-rs/tokio) | Tokio project | MIT |
| [Askama](https://github.com/djc/askama) | Dirkjan Ochtman | MIT / Apache 2.0 |
| [HTMX](https://htmx.org) | Carson Gross | BSD 2-Clause |
| [rumqttc](https://github.com/bytebeamio/rumqtt) | Bytebeam | Apache 2.0 |
| [Eclipse Mosquitto](https://mosquitto.org) | Eclipse Foundation | EPL 2.0 |
| [serde](https://serde.rs) | David Tolnay & Erick Tryzelaar | MIT / Apache 2.0 |

### Contributors

See [GitHub Contributors](https://github.com/boernmaster/RustyLox/graphs/contributors) for the full list.

---

## License

Apache License 2.0 — same as the original [LoxBerry](https://github.com/mschlenstedt/Loxberry) project.

```
Copyright 2024-2026 RustyLox Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0
```
