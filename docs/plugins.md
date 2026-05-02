# Plugin Development

RustyLox is compatible with the LoxBerry plugin format. A plugin is a ZIP archive containing a `plugin.cfg` manifest and optional hook scripts, daemon processes, and web frontend files.

---

## Plugin ZIP Structure

```
my-plugin.zip
├── plugin.cfg              # Required — plugin metadata
├── preroot.sh             # Optional — runs as root before install
├── preinstall.sh          # Optional — runs as loxberry user before install
├── preupgrade.sh          # Optional — runs as loxberry user before upgrade
├── postinstall.sh         # Optional — runs after install
├── postupgrade.sh         # Optional — runs after upgrade
├── postroot.sh            # Optional — runs as root after install/upgrade
├── uninstall.sh           # Optional — runs during uninstall
├── daemon/
│   └── daemon.pl          # Optional — background daemon process
└── webfrontend/
    └── htmlauth/
        └── index.html     # Optional — plugin web UI (requires auth)
```

---

## plugin.cfg

Required file in INI format:

```ini
[AUTHOR]
NAME=John Doe
EMAIL=john@example.com

[PLUGIN]
NAME=myplugin
FOLDER=myplugin
VERSION=1.0.0
TITLE_EN=My Plugin
TITLE_DE=Mein Plugin

[SYSTEM]
REBOOT=false
ARCHITECTURE=all
```

### Fields

| Section | Key | Description |
|---------|-----|-------------|
| `AUTHOR` | `NAME` | Author's full name |
| `AUTHOR` | `EMAIL` | Author's email |
| `PLUGIN` | `NAME` | Internal plugin name (alphanumeric, no spaces) |
| `PLUGIN` | `FOLDER` | Directory name under `plugins/` (usually same as `NAME`) |
| `PLUGIN` | `VERSION` | Semver string (e.g. `1.0.0`) |
| `PLUGIN` | `TITLE_EN` | English display title |
| `PLUGIN` | `TITLE_DE` | German display title |
| `SYSTEM` | `REBOOT` | `true` if a reboot is required after install |
| `SYSTEM` | `ARCHITECTURE` | `all`, `armhf`, `amd64`, etc. |

---

## Lifecycle Hooks

Hook scripts are executed at specific points during install, upgrade, and uninstall. They run in a subprocess with plugin environment variables pre-injected.

| Script | Runs as | When |
|--------|---------|------|
| `preroot.sh` | root | Before extraction during install/upgrade |
| `preinstall.sh` | loxberry | After extraction, before install completes |
| `preupgrade.sh` | loxberry | Before an upgrade starts |
| `postinstall.sh` | loxberry | After install completes |
| `postupgrade.sh` | loxberry | After upgrade completes |
| `postroot.sh` | root | After install/upgrade (root cleanup) |
| `uninstall.sh` | loxberry | During uninstall |

All hook scripts must be executable (`chmod +x`). Non-zero exit code is treated as a failure.

---

## Environment Variables Injected by RustyLox

Every hook script and daemon receives these variables:

| Variable | Example value | Description |
|----------|--------------|-------------|
| `LBHOMEDIR` | `/opt/loxberry` | LoxBerry home directory |
| `LBPPLUGINDIR` | `myplugin` | Plugin folder name |
| `LBPHTMLDIR` | `/opt/loxberry/webfrontend/html/plugins/myplugin` | Plugin public web dir |
| `LBPHTMLAUTHDIR` | `/opt/loxberry/webfrontend/htmlauth/plugins/myplugin` | Plugin auth web dir |
| `LBPDATADIR` | `/opt/loxberry/data/plugins/myplugin` | Plugin data dir |
| `LBPLOGDIR` | `/opt/loxberry/log/plugins/myplugin` | Plugin log dir |
| `LBPCONFIGDIR` | `/opt/loxberry/config/plugins/myplugin` | Plugin config dir |

---

## SDK Libraries

The LoxBerry Perl, PHP, and Bash SDK libraries are available inside the container at `/opt/loxberry/libs/`. They provide:

- Miniserver communication helpers
- MQTT publish/subscribe wrappers
- Logging utilities
- Config file read/write helpers

See the [original LoxBerry SDK documentation](https://wiki.loxberry.de/) for full library reference.

---

## Plugin Identity

Each plugin is uniquely identified by the MD5 hash of `AUTHOR.NAME + PLUGIN.NAME + PLUGIN.FOLDER`. This MD5 is used as the key in `plugindatabase.json` and in API paths (`/api/plugins/:md5`).

---

## Installing a Plugin

Via the web UI:
1. Navigate to **Plugins → Install Plugin**
2. Upload the `.zip` file

Via the API:
```bash
curl -X POST http://localhost/api/plugins/install \
  -F "file=@my-plugin.zip"
```

---

## Example: Minimal Bash Plugin

**plugin.cfg**
```ini
[AUTHOR]
NAME=Example Author
EMAIL=example@example.com

[PLUGIN]
NAME=hello
FOLDER=hello
VERSION=1.0.0
TITLE_EN=Hello Plugin

[SYSTEM]
REBOOT=false
ARCHITECTURE=all
```

**postinstall.sh**
```bash
#!/bin/bash
echo "Hello plugin installed to $LBPCONFIGDIR"
mkdir -p "$LBPDATADIR"
exit 0
```

---

## Daemon Process

If your plugin needs a persistent background process, place it in `daemon/`. RustyLox exposes daemon control via the API:

```bash
# Check status
curl http://localhost/api/plugins/hello/daemon/status

# Start
curl -X POST http://localhost/api/plugins/hello/daemon/start

# Stop
curl -X POST http://localhost/api/plugins/hello/daemon/stop
```

The daemon script is executed with the same environment variables as hook scripts.

---

## Plugin Web Frontend

Files placed in `webfrontend/htmlauth/` are served at:
```
http://localhost/plugins/<folder>/
```

This path requires authentication. To serve public (unauthenticated) files, use `webfrontend/html/`.

Plugin web UIs can call back into the RustyLox REST API or use the LoxBerry PHP SDK to interact with the Miniserver.

---

## Further Reading

- [Vitoconnect Plugin Integration](vitoconnect-integration.md) — real-world example of a PHP plugin with MQTT and Miniserver communication
- [Loxone API Reference](loxone-api/README.md) — Miniserver HTTP/WebSocket API
