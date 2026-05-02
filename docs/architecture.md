# Architecture

## Concept

RustyLox is LoxBerry as a Docker container. The original [LoxBerry](https://github.com/mschlenstedt/Loxberry) runs on a bare-metal Raspberry Pi and is built from Perl, PHP, and Bash scripts hosted by Apache. RustyLox replaces the entire stack with a single compiled Rust binary in a Docker image — same features, same LoxBerry directory layout, same environment variables for plugins — but deployable anywhere Docker runs.

The plugin compatibility layer ships the LoxBerry Perl/PHP/Bash SDK libraries inside the container so existing plugin hooks can execute unchanged. **Plugin compatibility is a work in progress.** The infrastructure (ZIP install, lifecycle hooks, SDK paths, env vars) is fully implemented, but individual plugins may rely on LoxBerry internals that are not yet replicated. Test reports and bug reports are welcome.

## Overview

RustyLox is a Cargo workspace containing 13 crates compiled into a single `rustylox-daemon` binary. All services run inside one Docker container alongside an Eclipse Mosquitto MQTT broker.

```
Browser / REST client
        |
        v
  rustylox-daemon  (Tokio async runtime)
  ┌─────────────────────────────────────────┐
  │  web-ui       Askama templates + HTMX   │
  │  web-api      Axum REST API             │
  │  auth         JWT + RBAC + API keys     │
  │  mqtt-gateway MQTT client + UDP         │
  │  miniserver-client  HTTP/UDP to Loxone  │
  │  plugin-manager     ZIP install/hooks   │
  │  metrics      sysinfo collection        │
  │  email-manager      SMTP                │
  │  task-scheduler     cron-like jobs      │
  │  backup-manager     ZIP archives        │
  │  rustylox-config    JSON config         │
  │  rustylox-logging   tracing wrapper     │
  │  rustylox-core      types & errors      │
  └─────────────────────────────────────────┘
        |                   |
        v                   v
  Mosquitto MQTT       Loxone Miniserver
```

## Crate Reference

| Crate | Purpose |
|-------|---------|
| `rustylox-core` | Shared types, error enum, `Result<T>` alias |
| `rustylox-config` | JSON config management — atomic writes, hot-reload |
| `rustylox-logging` | `tracing` subscriber setup, log rotation |
| `miniserver-client` | HTTP Basic Auth + UDP client for Loxone Miniserver; delta-send, CloudDNS, reboot detection |
| `mqtt-gateway` | rumqttc client, UDP listener (port 11884), transformer pipeline, MQTT↔Miniserver relay |
| `plugin-manager` | ZIP extraction, plugin DB (`plugindatabase.json`), lifecycle hook execution |
| `auth` | JWT HS256 signing, RBAC roles, Argon2id hashing, API key management, audit log |
| `metrics` | sysinfo wrapper — CPU, memory, disk, network interfaces |
| `email-manager` | SMTP send via lettre, send history persistence |
| `task-scheduler` | Cron-expression task runner, execution history |
| `backup-manager` | ZIP backup/restore of `config/`, `data/`, `log/` |
| `web-api` | Axum router with all REST API handlers, Prometheus metrics endpoint |
| `web-ui` | Askama templates, HTMX-driven UI handlers, SSE streams |
| `rustylox-daemon` | `main()` — wires all crates together, starts Tokio runtime |

### Dependency order (bottom-up)

```
rustylox-daemon
├── web-ui
│   └── web-api
│       ├── mqtt-gateway
│       ├── plugin-manager
│       ├── miniserver-client
│       ├── auth
│       ├── metrics
│       ├── email-manager
│       ├── task-scheduler
│       ├── backup-manager
│       └── rustylox-config
│           └── rustylox-core
└── rustylox-logging
```

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust 1.80+, Edition 2021 |
| Async runtime | Tokio |
| Web framework | Axum 0.7 |
| Templates | Askama (compiled, type-checked) |
| Frontend | HTMX (no SPA, no bundler) |
| MQTT client | rumqttc 0.24 |
| MQTT broker | Eclipse Mosquitto 2.0 |
| Serialization | serde / serde_json / serde_ini |
| Concurrent maps | DashMap |
| Password hashing | Argon2id |
| Auth tokens | JWT HS256 |
| Metrics | Prometheus (tower-http) |
| Email | lettre (SMTP) |
| Deployment | Docker, multi-stage build |
| CI/CD | GitHub Actions |

## Storage

There is no SQL database. All persistence is JSON file-backed with atomic temp-file-then-rename writes:

| File | Contents |
|------|----------|
| `config/system/general.json` | Main system configuration |
| `config/system/mqtt_subscriptions.cfg` | MQTT subscription topics (INI format) |
| `config/system/mqtt_transformers.cfg` | Transformer pipeline config (INI format) |
| `data/system/plugindatabase.json` | Installed plugin registry |
| `data/system/auth.json` | Users, API keys, audit log |
| `data/system/email_history.json` | Email send history |
| `data/system/task_history.json` | Task execution history |

All paths above are relative to `$LBHOMEDIR` (default `/opt/loxberry`).

## Differences from Original LoxBerry

| Aspect | Original LoxBerry | RustyLox |
|--------|------------------|---------|
| Language | Perl / PHP / Bash | Rust |
| Runtime | Interpreted scripts | Single compiled binary |
| Deployment | Bare-metal Raspberry Pi | Docker container |
| Async | Synchronous CGI | Tokio async runtime |
| Web | Apache + CGI | Axum + Askama + HTMX |
| Config format | INI / custom | JSON (compatible subset) |
| Plugin SDK | Native Perl/PHP/Bash | Same SDK libraries bundled in Docker image; hooks execute the same way — individual plugin compatibility varies (WIP) |
| MQTT | LoxBerry MQTT GW plugin | Built-in |
| Auth | Basic Auth / session files | JWT + RBAC |
