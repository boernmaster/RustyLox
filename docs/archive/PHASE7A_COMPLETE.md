# Phase 7a: Complete Web UI for Backend Features

![Status](https://img.shields.io/badge/Status-Complete-brightgreen)
![Phase](https://img.shields.io/badge/Phase-7a-brightgreen)
![Completion](https://img.shields.io/badge/Completion-100%25-brightgreen)

## Overview

Phase 7a builds a full web UI for all backend functionality introduced in Phases 5-7, and expands Miniserver backup with a full recursive download and real-time progress bar.

---

## Deliverables

### Authentication
- [x] `/login`, `/logout` — JWT cookie flow with redirect-after-login
- [x] `/profile` — User profile page
- [x] Auth middleware enforces login across entire web UI (public paths: `/login`, `/static/`, `/health`)

### Admin
- [x] `/admin/users` — User management (list, create, edit, delete)
- [x] `/admin/api-keys` — API key management (create, revoke, copy)
- [x] `/admin/audit` — Audit log viewer with filters
- [x] `/admin/security` — Security settings (lockout, password policy, sessions)
- [x] `/admin/database` — Database management (status, migrations, table browser)

### Monitoring & Operations
- [x] `/system-health` — System health dashboard (CPU, memory, disk, service status)
- [x] `/email` — Email configuration (SMTP settings, test send, send history with status badges)
- [x] `/tasks` — Task scheduler (list, enable/disable, manual run, execution history with duration/status)
- [x] `/network` — Network diagnostics (ping, port scan, connectivity tests)
- [x] `/backup` — Backup & restore UI
- [x] `/system-update` — Check GitHub releases, view release notes and assets, update instructions

### Miniserver Backup (`/miniserver/backup`)
- [x] Full recursive backup of all data directories: `log`, `prog`, `sys`, `stats`, `temp`, `update`, `web`, `user`
- [x] BFS directory walk via `/dev/fslist/` — all subdirectories followed automatically
- [x] All files packed into a single timestamped ZIP archive
- [x] Automatic backup scheduling (configurable interval: 6h / 12h / 24h / 48h / weekly)
- [x] Real-time SSE progress bar — file count, current filename, animated fill
- [x] Background task spawned immediately; browser connects via `EventSource`
- [x] 7 most recent backups kept per Miniserver

### Data Persistence
- [x] Email send history — JSON file at `data/system/email_history.json` (max 100 entries, newest-first)
- [x] Task execution history — JSON file at `data/system/task_history.json` (fixes stateless-per-request scheduler)

### CSS & Accessibility
- [x] Skip-to-content link for keyboard navigation
- [x] `:focus-visible` outlines on all interactive elements
- [x] Mobile hamburger nav toggle (`.nav-toggle` + `.nav-menu.open`)
- [x] `.sr-only` screen-reader-only utility class
- [x] Print styles (`@media print`)
- [x] 480px small-screen breakpoint
- [x] `.badge-info`, `.text-muted` utility classes
- [x] Smooth transitions on form inputs

### Compatibility
- [x] `/admin/system/tools/logfile.cgi` — LoxBerry-compatible log viewer route for plugins
- [x] `/weather`, `/api-docs` — bonus pages

---

## Architecture

- **Templates**: Askama (type-safe, server-rendered)
- **Interactivity**: HTMX — no SPA, progressive enhancement
- **Real-time**: SSE (`axum::response::Sse` + `async_stream::stream!`)
- **Progress tracking**: `tokio::sync::mpsc::unbounded_channel` + global `LazyLock<Mutex<HashMap<job_id, Receiver>>>`
- **History persistence**: JSON files in `data/system/` with async tokio I/O
- **Update check**: GitHub Releases API via `reqwest` with 15s timeout

---

## Pages Summary

| Route | Purpose |
|-------|---------|
| `/login` | JWT cookie authentication |
| `/logout` | Session termination |
| `/` | Dashboard |
| `/profile` | User profile |
| `/miniserver` | Miniserver overview |
| `/miniserver/monitor` | Communication monitor |
| `/miniserver/backup` | Miniserver backup with SSE progress |
| `/mqtt/monitor` | MQTT real-time monitor |
| `/mqtt/config` | MQTT configuration |
| `/mqtt/stats` | MQTT statistics |
| `/weather` | Weather current & forecast |
| `/weather/config` | Weather configuration |
| `/plugins` | Installed plugins |
| `/plugins/install` | Plugin installer |
| `/system-health` | System health dashboard |
| `/logs` | Log viewer |
| `/backup` | Backup & restore |
| `/tasks` | Task scheduler + execution history |
| `/network` | Network diagnostics |
| `/email` | Email config + send history |
| `/system-update` | System update checker |
| `/settings` | System settings |
| `/admin/users` | User management |
| `/admin/api-keys` | API key management |
| `/admin/audit` | Audit log |
| `/admin/security` | Security settings |
| `/admin/database` | Database management |
| `/api-docs` | API documentation |
