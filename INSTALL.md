# RustyLox Installation Guide

## Requirements

| Requirement | Minimum Version | Notes |
|------------|----------------|-------|
| Rust | 1.80+ | Install via [rustup](https://rustup.rs/) |
| Docker | 24+ | For containerised deployment |
| Docker Compose | 2.20+ | Included with Docker Desktop |
| Git | 2.x | For cloning the repository |

---

## Quick Start (Docker — Recommended)

```bash
# 1. Clone the repository
git clone https://github.com/boernmaster/RustyLox.git
cd RustyLox

# 2. Create volume directories
mkdir -p volumes/config/system volumes/data/system volumes/log/system

# 3. Build and start services
docker compose up -d

# 4. Check that the service is running
curl http://localhost:8080/health
# Expected: {"status":"ok"}

# 5. Open the web UI
open http://localhost:8080
```

The stack starts two containers:
- **loxberry** – the main RustyLox daemon (port 8080)
- **mosquitto** – Eclipse Mosquitto MQTT broker (port 1883)

---

## Local Development (No Docker)

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone repository
git clone https://github.com/boernmaster/RustyLox.git
cd RustyLox

# Create config directories
mkdir -p /tmp/loxberry/config/system \
         /tmp/loxberry/data/system   \
         /tmp/loxberry/log/system    \
         /tmp/loxberry/static

# Copy static assets
cp -r static /tmp/loxberry/

# Build
cargo build --release

# Run
LBHOMEDIR=/tmp/loxberry RUST_LOG=info \
  ./target/release/loxberry-daemon
```

---

## Configuration

### Initial Setup

On first run with no existing `general.json`, RustyLox creates a default
configuration. Copy the example file to get started:

```bash
cp volumes/config/system/general.json.example \
   volumes/config/system/general.json   # if an example exists
# or let the daemon create it on first start
```

### Key Configuration Options

All configuration lives in `volumes/config/system/general.json`.

| Section | Key | Default | Description |
|---------|-----|---------|-------------|
| `Base` | `Lang` | `en` | UI language (en, de, fr, es) |
| `Base` | `Systemloglevel` | `6` | Log verbosity (1=error … 7=trace) |
| `Mqtt` | `Brokerhost` | `mosquitto` | MQTT broker hostname |
| `Mqtt` | `Brokerport` | `1883` | MQTT broker port |
| `Mqtt` | `Udpinport` | `11884` | UDP input port (0 = disabled) |
| `Backup` | `Schedule.Active` | `false` | Enable scheduled backups |
| `Backup` | `Schedule.IntervalHours` | `24` | Hours between backups |
| `Backup` | `Schedule.KeepBackups` | `7` | Number of backups to retain |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LBHOMEDIR` | `/opt/loxberry` | LoxBerry home directory |
| `RUST_LOG` | `info` | Tracing log level |
| `BIND_ADDR` | `0.0.0.0:8080` | HTTP bind address |

---

## Volume Mounts (Docker)

| Host Path | Container Path | Purpose |
|-----------|---------------|---------|
| `./volumes/config` | `/opt/loxberry/config` | JSON and INI configuration files |
| `./volumes/data`   | `/opt/loxberry/data`   | Plugin data and backup archives |
| `./volumes/log`    | `/opt/loxberry/log`    | Log files |

---

## Ports

| Port | Protocol | Service |
|------|----------|---------|
| 8080 | TCP | RustyLox web UI and REST API |
| 1883 | TCP | MQTT broker (Mosquitto) |
| 11884 | UDP | MQTT UDP input gateway |

---

## Plugin Installation

Plugins are ZIP archives containing a `plugin.cfg` manifest.

1. Navigate to **Plugins → Install Plugin** in the web UI.
2. Upload the `.zip` file.
3. The system extracts the archive, runs `preinstall.sh` / `postinstall.sh`
   hooks, and registers the plugin in `plugindatabase.json`.

Plugin hooks can use the full LoxBerry Perl/PHP/Bash SDK. Libraries are
pre-installed at `/opt/loxberry/libs/`.

---

## Backup and Restore

### Manual Backup

```bash
curl -X POST http://localhost:8080/api/backup/create
# Response: {"success":true,"backup_name":"loxberry_backup_20260316_120000.tar.gz"}
```

### Restore

```bash
curl -X POST http://localhost:8080/api/backup/loxberry_backup_20260316_120000.tar.gz/restore
```

Or use the **Backup** page in the web UI.

---

## Updating RustyLox

```bash
git pull origin main
docker compose build
docker compose up -d
```

---

## Troubleshooting

### Service won't start

```bash
docker compose logs loxberry
```

Look for `ERROR` lines. Common causes:
- Missing or invalid `general.json` — delete it to regenerate defaults.
- Port 8080 already in use — change `BIND_ADDR` in `docker-compose.yml`.

### MQTT messages not appearing

1. Check the broker is running: `docker compose ps mosquitto`
2. Verify `Brokerhost` and `Brokerport` in Settings → MQTT Config.
3. Check subscriptions file: `volumes/config/system/mqtt_subscriptions.cfg`.

### Plugin hook fails

1. Check plugin logs in `volumes/log/plugins/<plugin-name>/`.
2. Verify Perl/PHP is available: `docker compose exec loxberry perl -v`
3. Ensure `LBHOMEDIR` is correctly set to `/opt/loxberry` inside the container.

---

## Uninstalling

```bash
docker compose down -v   # removes containers and named volumes
rm -rf volumes/          # removes all persistent data (irreversible)
```
