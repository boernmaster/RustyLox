# REST API Reference

Interactive documentation is also available at **`/api-docs`** in the running web UI.

All API endpoints are under `/api/*`. Endpoints that modify state require authentication (JWT cookie or `Authorization: Bearer <api-key>` header). Read-only endpoints (health, metrics) are public.

---

## Health & Metrics

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Simple liveness check. Returns `{"status":"ok"}` |
| `GET` | `/api/health/detail` | Per-component health: `config`, `mqtt_broker`, `miniserver`, `disk_space`, `cpu`, `memory` |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/api/system/metrics` | System metrics as JSON |

---

## System

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/system/status` | System status, version, uptime |
| `GET` | `/api/system/log-level` | Current log level |
| `PUT` | `/api/system/log-level` | Set log level at runtime |
| `GET` | `/api/system/update/check` | Check for a newer GitHub release |
| `POST` | `/api/system/update/apply` | Apply pending update |

---

## Configuration

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/config/general` | Read `general.json` |
| `PUT` | `/api/config/general` | Update `general.json` |

---

## Miniserver

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/miniserver` | List all configured Miniservers |
| `GET` | `/api/miniserver/:id` | Get Miniserver details |
| `GET` | `/api/miniserver/:id/status` | Test connection status |
| `POST` | `/api/miniserver/:id/send` | Send HTTP command to Miniserver |
| `POST` | `/api/miniserver/:id/get` | Get values from Miniserver |

---

## MQTT Gateway

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/mqtt/status` | Gateway connection status and stats |
| `GET` | `/api/mqtt/relayed-topics` | Relay cache — last forwarded value per topic |
| `GET` | `/api/mqtt/finder` | Recent messages per topic (MQTT Finder view) |
| `POST` | `/api/mqtt/subscriptions/reload` | Hot-reload `mqtt_subscriptions.cfg` |
| `POST` | `/api/mqtt/transformers/reload` | Hot-reload `mqtt_transformers.cfg` |
| `GET` | `/api/mqtt/stats` | Message counters |
| `POST` | `/api/mqtt/stats/reset` | Reset message counters |

---

## Plugins

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/plugins` | List all installed plugins |
| `GET` | `/api/plugins/:md5` | Get plugin details |
| `POST` | `/api/plugins/install` | Install plugin from ZIP (multipart upload) |
| `DELETE` | `/api/plugins/:md5` | Uninstall plugin |
| `POST` | `/api/plugins/:md5/upgrade` | Upgrade plugin from ZIP |
| `GET` | `/api/plugins/:folder/daemon/status` | Plugin daemon status |
| `POST` | `/api/plugins/:folder/daemon/start` | Start plugin daemon |
| `POST` | `/api/plugins/:folder/daemon/stop` | Stop plugin daemon |
| `POST` | `/api/plugins/:folder/daemon/restart` | Restart plugin daemon |

---

## Backup

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/backup` | List available backups |
| `POST` | `/api/backup/create` | Create a new backup immediately |
| `GET` | `/api/backup/:name/download` | Download backup archive |
| `POST` | `/api/backup/:name/restore` | Restore from backup |
| `DELETE` | `/api/backup/:name` | Delete backup |
| `GET` | `/api/backup/schedule` | Get backup schedule config |
| `PUT` | `/api/backup/schedule` | Update backup schedule config |

---

## Scheduled Tasks

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/tasks` | List all scheduled tasks |
| `POST` | `/api/tasks` | Create a task |
| `PUT` | `/api/tasks/:id` | Update a task |
| `DELETE` | `/api/tasks/:id` | Delete a task |
| `POST` | `/api/tasks/:id/run` | Trigger task immediately |
| `GET` | `/api/tasks/history` | Execution history |

Built-in tasks: `backup`, `log_rotation`, `health_check`, `miniserver_backup`.

---

## Network Diagnostics

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/network/ping` | Ping a host |
| `GET` | `/api/network/interfaces` | List network interfaces (IP, MAC, status) |
| `POST` | `/api/network/test/connection` | TCP connectivity test |
| `POST` | `/api/network/test/miniserver` | Test Miniserver reachability |
| `POST` | `/api/network/test/mqtt` | Test MQTT broker reachability |

---

## Email

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/email/config` | Get SMTP configuration |
| `PUT` | `/api/email/config` | Update SMTP configuration |
| `POST` | `/api/email/test` | Send a test email |
| `POST` | `/api/email/send` | Send a notification email |
| `GET` | `/api/email/history` | Send history |

---

## Weather

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/weather/current` | Current weather data |
| `GET` | `/api/weather/forecast` | Daily forecast |
| `GET` | `/api/weather/hourly` | Hourly forecast |
| `GET` | `/api/weather/all` | All weather data |
| `GET` | `/api/weather/config` | Weather service configuration |
| `PUT` | `/api/weather/config` | Update weather configuration |
| `POST` | `/api/weather/refresh` | Force data refresh |

---

## Authentication & Users

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/auth/login` | Log in — sets `lb_token` JWT cookie |
| `POST` | `/api/auth/logout` | Log out — clears cookie |
| `GET` | `/api/auth/me` | Current user info |
| `GET` | `/api/auth/keys` | List API keys |
| `POST` | `/api/auth/keys` | Create API key |
| `DELETE` | `/api/auth/keys/:id` | Revoke API key |
| `GET` | `/api/auth/audit` | Audit log |
| `GET` | `/api/users` | List users (admin only) |
| `POST` | `/api/users` | Create user (admin only) |
| `DELETE` | `/api/users/:id` | Delete user (admin only) |
| `PUT` | `/api/users/:id/password` | Change password |

### Roles

| Role | Capabilities |
|------|-------------|
| `Admin` | Full access including user management |
| `Operator` | Read/write access to all features except user management |
| `Viewer` | Read-only access |
| `PluginManager` | Plugin install/uninstall only |

### API Key Authentication

Include the key in the `Authorization` header:

```
Authorization: Bearer lbx_<key>
```

Keys are stored as SHA-256 hashes and are shown only once at creation time.

---

## Virtual HTTP Input (Miniserver → RustyLox)

The Miniserver can push values to RustyLox using its Virtual HTTP Output:

```
http://<user>:<pass>@<RUSTYLOX_IP>/input/vi?name=<vi_name>&value=<value>
```

This endpoint does **not** require JWT; it uses HTTP Basic Auth matching the configured Miniserver credentials.
