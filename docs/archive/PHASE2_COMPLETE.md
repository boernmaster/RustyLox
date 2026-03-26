# Phase 2 Complete: Plugin Management System ✅

<div align="center">

![Status](https://img.shields.io/badge/Status-Complete-success)
![Phase](https://img.shields.io/badge/Phase-2-blue)
![Features](https://img.shields.io/badge/Features-Plugin%20System-green)

</div>

## Overview

Phase 2 implements a complete plugin management system for LoxBerry Rust, including:
- ✅ Plugin database with JSON persistence
- ✅ ZIP extraction and installation lifecycle
- ✅ Lifecycle hook execution (preroot, preinstall, postinstall, postroot, uninstall)
- ✅ Directory isolation for plugins
- ✅ REST API endpoints for plugin management
- ✅ Sample plugin for testing

## Implementation Details

### 1. Plugin Manager Crate (`crates/plugin-manager/`)

#### Core Modules

**database.rs** - Plugin Database Management
- `PluginDatabase`: HashMap-based storage keyed by MD5 hash
- `PluginEntry`: Complete plugin metadata (name, version, author, directories, timestamps)
- JSON persistence to `data/system/plugindatabase.json`
- Query methods: `find_by_md5()`, `find_by_folder()`, `find_by_name()`
- MD5 calculation: `MD5(author_name + author_email + name + folder)`

**config_parser.rs** - Plugin Configuration Parser
- Parses `plugin.cfg` in INI format
- Sections: `[AUTHOR]`, `[PLUGIN]`, `[SYSTEM]`, `[DAEMON]`, `[CRON]`, `[SUDOERS]`, `[APT]`
- Handles multilingual titles (TITLE_EN, TITLE_DE, etc.)
- Extracts optional system requirements and service configurations

**directory_manager.rs** - Directory Isolation
- Creates isolated directory structure for each plugin:
  - `webfrontend/htmlauth/plugins/{folder}/` - Authenticated web interface
  - `webfrontend/html/plugins/{folder}/` - Public web content
  - `templates/plugins/{folder}/` - Templates and language files
  - `data/plugins/{folder}/` - Plugin data storage
  - `log/plugins/{folder}/` - Plugin logs
  - `config/plugins/{folder}/` - Plugin configuration
  - `bin/plugins/{folder}/` - Plugin binaries/scripts
- Automatic cleanup on uninstall

**lifecycle.rs** - Lifecycle Hook Execution
- Hook types: PreRoot, PreInstall, PostInstall, PostRoot, PostUpgrade, Uninstall
- Executes bash scripts via `tokio::process::Command`
- Environment variable injection for SDK compatibility:
  - `LBHOMEDIR`, `LBPPLUGINDIR`, `LBPDATADIR`, `LBPLOGDIR`, etc.
- Returns `HookResult` with stdout, stderr, exit code
- Makes scripts executable (`chmod 755`)

**installer.rs** - Main Installation Orchestrator
- Installation actions: Install, Upgrade, Reinstall
- Complete workflow:
  1. Extract ZIP to temp directory
  2. Parse plugin.cfg
  3. Calculate MD5 checksum
  4. Check for existing plugin (version conflict detection)
  5. Execute preroot hook (as root)
  6. Execute preinstall hook (as loxberry user)
  7. Create plugin directory structure
  8. Copy files to isolated directories with permissions
  9. Execute postinstall hook
  10. Execute postroot hook
  11. Update plugin database with entry
  12. Save database to disk
- Uninstallation workflow:
  1. Find plugin in database
  2. Execute uninstall hook
  3. Remove all plugin directories
  4. Remove from database
  5. Save database

### 2. Web API Endpoints (`crates/web-api/src/routes/plugins.rs`)

#### Endpoints

**GET /api/plugins** - List all installed plugins
- Returns: Array of `PluginEntry` objects with count
- Empty array if no plugins installed

**GET /api/plugins/:md5** - Get plugin details
- Parameter: MD5 hash of plugin
- Returns: Full `PluginEntry` object
- 404 if not found

**POST /api/plugins/install** - Install plugin from ZIP
- Content-Type: `multipart/form-data`
- Field: `file` (ZIP archive)
- Workflow:
  1. Receive uploaded ZIP file
  2. Save to temporary file
  3. Pass to `PluginInstaller::install()`
  4. Return installed plugin details or error
- Returns: `PluginInstallResponse` with success/error

**DELETE /api/plugins/:md5** - Uninstall plugin
- Parameter: MD5 hash of plugin
- Executes uninstall hook before removal
- Returns: Success/error message

**POST /api/plugins/:md5/upgrade** - Upgrade plugin
- Content-Type: `multipart/form-data`
- Field: `file` (new version ZIP)
- Preserves install timestamp, updates update timestamp
- Returns: Updated plugin details or error

### 3. Application State Updates

**AppState** now includes:
- `lbhomedir: PathBuf` - LoxBerry home directory (typically `/opt/loxberry`)
- Available to all route handlers
- Passed to `PluginInstaller` for directory operations

### 4. Sample Plugin (`examples/sample-plugin/`)

Complete test plugin including:
- `plugin.cfg` - Properly formatted configuration
- `preinstall.sh` - Creates config file with install timestamp
- `postinstall.sh` - Creates sample data file
- `uninstall.sh` - Cleanup operations
- `webfrontend/htmlauth/index.html` - Simple web interface
- `README.md` - Complete testing instructions

## Testing

### Build Test Plugin

```bash
cd examples/sample-plugin
zip -r ../sample-plugin.zip .
cd ../..
```

### Start LoxBerry

```bash
docker-compose up -d
```

### Install Plugin

```bash
curl -X POST -F 'file=@examples/sample-plugin.zip' \
  http://localhost:8080/api/plugins/install | jq '.'
```

### List Plugins

```bash
curl http://localhost:8080/api/plugins | jq '.'
```

### Get Plugin Details

```bash
# Use MD5 from list response
curl http://localhost:8080/api/plugins/MD5_HASH | jq '.'
```

### Verify Installation

```bash
# Check plugin directories
docker exec rustylox ls -la /opt/loxberry/data/plugins/sampleplugin
docker exec rustylox ls -la /opt/loxberry/config/plugins/sampleplugin
docker exec rustylox ls -la /opt/loxberry/webfrontend/htmlauth/plugins/sampleplugin

# Check database
docker exec rustylox cat /opt/loxberry/data/system/plugindatabase.json | jq '.'

# Check hook execution (should see install.cfg from preinstall hook)
docker exec rustylox cat /opt/loxberry/config/plugins/sampleplugin/install.cfg

# Check sample data (from postinstall hook)
docker exec rustylox cat /opt/loxberry/data/plugins/sampleplugin/sample.txt
```

### Uninstall Plugin

```bash
curl -X DELETE http://localhost:8080/api/plugins/MD5_HASH
```

### Verify Cleanup

```bash
# Directories should be gone
docker exec rustylox ls -la /opt/loxberry/data/plugins/
docker exec rustylox ls -la /opt/loxberry/config/plugins/

# Database should be empty
docker exec rustylox cat /opt/loxberry/data/system/plugindatabase.json | jq '.'
```

## Plugin Configuration Format

### Minimal plugin.cfg

```ini
[AUTHOR]
NAME=Your Name
EMAIL=you@example.com

[PLUGIN]
NAME=MyPlugin
FOLDER=myplugin
VERSION=1.0.0
TITLE_EN=My Plugin
```

### Full plugin.cfg Example

```ini
[AUTHOR]
NAME=Test Author
EMAIL=test@example.com

[PLUGIN]
NAME=SamplePlugin
FOLDER=sampleplugin
VERSION=1.0.0
TITLE_EN=Sample Plugin
TITLE_DE=Beispiel Plugin
INTERFACE=index.html
AUTOUPDATE=2
LOGLEVEL=6

[SYSTEM]
LB_MINIMUM=3.0.0
LB_MAXIMUM=4.0.0

[DAEMON]
DAEMON=/opt/loxberry/bin/plugins/sampleplugin/daemon.sh
ENABLED=0

[CRON]
SCHEDULE=*/5 * * * *
COMMAND=/opt/loxberry/bin/plugins/sampleplugin/cron.sh
ENABLED=0

[SUDOERS]
COMMAND1=/bin/systemctl restart myservice
COMMAND2=/usr/bin/apt-get update

[APT]
PACKAGE1=python3-pip
PACKAGE2=mosquitto-clients
```

## Environment Variables Injected for Hooks

When lifecycle hooks are executed, these environment variables are set:

- `LBHOMEDIR` - LoxBerry home directory (`/opt/loxberry`)
- `LBPPLUGINDIR` - Plugin folder name
- `LBPHTMLAUTHDIR` - Authenticated web interface directory
- `LBPHTMLDIR` - Public web content directory
- `LBPTEMPLATEDIR` - Templates directory
- `LBPDATADIR` - Plugin data directory
- `LBPLOGDIR` - Plugin log directory
- `LBPCONFIGDIR` - Plugin config directory
- `LBPBINDIR` - Plugin binaries directory

## Plugin ZIP Structure

```
plugin-name.zip
├── plugin.cfg              (required)
├── preinstall.sh          (optional)
├── postinstall.sh         (optional)
├── uninstall.sh           (optional)
├── webfrontend/           (optional)
│   ├── htmlauth/          (authenticated pages)
│   │   └── index.html
│   └── html/              (public pages)
├── templates/             (optional)
│   └── lang/
│       ├── language_en.ini
│       └── language_de.ini
├── bin/                   (optional)
│   ├── daemon.sh
│   └── cron.sh
├── config/                (optional)
│   └── defaults.cfg
└── data/                  (optional)
```

## Database Schema

Plugin database is stored as JSON at `data/system/plugindatabase.json`:

```json
{
  "plugins": {
    "abc123def456...": {
      "md5": "abc123def456...",
      "author_name": "Test Author",
      "author_email": "test@example.com",
      "version": "1.0.0",
      "name": "SamplePlugin",
      "folder": "sampleplugin",
      "title": {
        "en": "Sample Plugin",
        "de": "Beispiel Plugin"
      },
      "interface": "index.html",
      "autoupdate": 2,
      "releasecfg": null,
      "prereleasecfg": null,
      "loglevel": "6",
      "directories": {
        "htmlauth": "/opt/loxberry/webfrontend/htmlauth/plugins/sampleplugin",
        "html": "/opt/loxberry/webfrontend/html/plugins/sampleplugin",
        "template": "/opt/loxberry/templates/plugins/sampleplugin",
        "data": "/opt/loxberry/data/plugins/sampleplugin",
        "log": "/opt/loxberry/log/plugins/sampleplugin",
        "config": "/opt/loxberry/config/plugins/sampleplugin",
        "bin": "/opt/loxberry/bin/plugins/sampleplugin"
      },
      "install_timestamp": 1678901234,
      "update_timestamp": 1678901234
    }
  }
}
```

## Technical Notes

### MD5 Hash Calculation
Plugins are uniquely identified by MD5 hash of: `author_name + author_email + name + folder`

This ensures:
- Same plugin by same author = same MD5
- Prevents conflicts between plugins with same name from different authors
- Stable identifier across versions (for upgrades)

### File Permissions
- Hook scripts are automatically made executable (`chmod 755`)
- All plugin files owned by `loxberry:loxberry` user (in production)
- Directories created with default permissions

### Temporary Files
- ZIP extracts to system temp directory (auto-cleanup)
- Upload uses `tempfile` crate for secure temp file handling
- Temp files deleted on error or completion

### Error Handling
- Preroot/Preinstall hook failures = installation aborted
- Postinstall/Postroot hook failures = logged as warnings but installation continues
- Uninstall hook failures = logged as warnings but uninstall continues
- Missing plugin.cfg = installation fails immediately
- Invalid plugin.cfg format = descriptive error returned

## Dependencies Added

```toml
# plugin-manager/Cargo.toml
zip = "2.1"            # ZIP extraction
serde_ini = "0.2"      # INI parsing
md5 = "0.7"            # MD5 checksum
walkdir = "2.4"        # Recursive directory operations
tempfile = "3.8"       # Secure temp files

# web-api/Cargo.toml
plugin-manager = { path = "../plugin-manager" }
tempfile = { workspace = true }
```

## Next Steps (Phase 3: MQTT Gateway)

- MQTT broker integration (rumqttc)
- UDP listener on port 11884
- Subscription management from plugin configs
- Message transformer pipeline (JSON expansion, boolean conversion, scripts)
- Relay to Miniserver via HTTP/UDP
- Hot-reload transformers on file changes

## Files Changed in Phase 2

```
Cargo.toml                                    # Added plugin-manager member, tempfile dependency
crates/loxberry-core/src/lib.rs              # Export PluginPaths
crates/plugin-manager/                        # New crate (5 modules)
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── database.rs                          # Plugin database
│   ├── config_parser.rs                     # plugin.cfg parser
│   ├── directory_manager.rs                 # Directory operations
│   ├── lifecycle.rs                         # Hook execution
│   └── installer.rs                         # Installation orchestrator
crates/web-api/src/
├── lib.rs                                   # Added plugin routes
├── state.rs                                 # Added lbhomedir field
└── routes/
    ├── mod.rs                               # Export plugins module
    └── plugins.rs                           # New plugin endpoints
crates/loxberry-daemon/src/main.rs           # Updated AppState::new()
examples/sample-plugin/                       # New test plugin
├── plugin.cfg
├── preinstall.sh
├── postinstall.sh
├── uninstall.sh
├── webfrontend/htmlauth/index.html
└── README.md
test-phase2.sh                               # Test script
PHASE2_COMPLETE.md                           # This file
```

## Verification Checklist

- [x] Plugin database CRUD operations
- [x] Plugin.cfg parsing (all sections)
- [x] MD5 checksum calculation
- [x] ZIP extraction
- [x] Directory structure creation
- [x] File copying with permissions
- [x] Lifecycle hook execution with environment variables
- [x] REST API endpoints (install, list, get, uninstall, upgrade)
- [x] Multipart file upload handling
- [x] Sample plugin for testing
- [x] Error handling and validation
- [x] Docker build integration
- [ ] End-to-end installation test (pending Docker build)
- [ ] Hook script execution test (pending Docker build)
- [ ] Database persistence test (pending Docker build)

## Known Limitations

1. **Root Hooks**: PreRoot and PostRoot hooks log warnings instead of actually executing as root (Docker limitation)
2. **APT Packages**: APT section parsed but installation not yet implemented
3. **Sudoers**: Sudoers section parsed but not yet applied to system
4. **Daemon**: Daemon section parsed but daemon management not yet implemented
5. **Cron**: Cron section parsed but cron job creation not yet implemented

These will be addressed in later phases or iterations.
