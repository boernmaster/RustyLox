# Phase 5+ Implementation Plan

<div align="center">

![Status](https://img.shields.io/badge/Status-Complete-green)
![Phase](https://img.shields.io/badge/Phase-5%2B-blue)
![Priority](https://img.shields.io/badge/Priority-SDK%20%26%20Logging-red)

</div>

## Overview
Implement SDK compatibility, logging, backup/restore, and polish existing features.

## 1. Logging Framework (Priority: HIGH)

### 1.1 Structured Logging System
- [x] Basic tracing-subscriber (already implemented)
- [x] Log rotation with tracing-appender (`crates/loxberry-logging/src/rotation.rs`)
- [x] Per-component log levels (e.g. `web_api=debug,mqtt_gateway=trace` via API and settings UI)
- [x] Plugin-specific log files (`crates/loxberry-logging/src/plugin_logger.rs`)
- [x] Web UI log viewer (`/logs` route with file selection and tail view)

**Files created:**
- `crates/loxberry-logging/src/lib.rs` - Logging crate
- `crates/loxberry-logging/src/rotation.rs` - Log rotation with retention policies
- `crates/loxberry-logging/src/plugin_logger.rs` - Plugin-specific logging

### 1.2 Log Management
- [x] Log level configuration via API (`GET/PUT /api/system/log-level`)
- [x] View logs via web UI (tail view with configurable line count)
- [x] Log search and filtering (real-time filter with highlight in `/logs` viewer)
- [x] Log retention policies (cleanup_logs with RotationPolicy)

## 2. SDK Compatibility Layer (Priority: HIGH)

### 2.1 Directory Structure
Copy LoxBerry SDK files to Docker image:
```
/opt/loxberry/
├── libs/
│   ├── perllib/LoxBerry/    - Perl modules
│   ├── phplib/               - PHP libraries
│   └── bashlib/              - Bash functions
├── bin/                      - User scripts
├── sbin/                     - System scripts
├── webfrontend/
│   ├── html/                 - Public web files
│   └── htmlauth/             - Authenticated web files
└── templates/                - Template files
```

**Implementation:**
- [x] Update Dockerfile to copy SDK libraries (`COPY sdk/perllib`, `phplib`, `bashlib`)
- [x] Create directory structure in container (`/opt/loxberry/libs/`, webfrontend dirs, etc.)
- [x] Set correct permissions (`chown -R loxberry:loxberry /opt/loxberry`)

### 2.2 Environment Variable Injection
**Files created:**
- [x] `crates/plugin-manager/src/environment.rs` - Full SDK environment builder

Environment variables to inject:
```bash
LBHOMEDIR=/opt/loxberry
LBPPLUGINDIR=plugin_folder
LBPHTMLDIR=/opt/loxberry/webfrontend/html/plugins/{folder}
LBPHTMLAUTHDIR=/opt/loxberry/webfrontend/htmlauth/plugins/{folder}
LBPTEMPLATEDIR=/opt/loxberry/templates/plugins/{folder}
LBPDATADIR=/opt/loxberry/data/plugins/{folder}
LBPLOGDIR=/opt/loxberry/log/plugins/{folder}
LBPCONFIGDIR=/opt/loxberry/config/plugins/{folder}
LBPBINDIR=/opt/loxberry/bin/plugins/{folder}
# ... all other SDK variables
```

### 2.3 Perl SDK Integration
- [ ] Test with real plugin (preinstall/postinstall hooks)
- [ ] Verify LoxBerry::System path detection works
- [ ] Verify LoxBerry::Log creates proper log files
- [ ] Test LoxBerry::IO Miniserver communication

### 2.4 Plugin Execution Wrapper
**Files created:**
- [x] `crates/plugin-manager/src/executor.rs` - Script executor with environment (Perl/PHP/Bash)

```rust
pub struct PluginExecutor {
    pub async fn execute_perl(&self, script: &Path, plugin: &PluginEntry) -> Result<Output>
    pub async fn execute_php(&self, script: &Path, plugin: &PluginEntry) -> Result<Output>
    pub async fn execute_bash(&self, script: &Path, plugin: &PluginEntry) -> Result<Output>
}
```

## 3. Backup & Restore (Priority: MEDIUM)

### 3.1 Backup System
**Files created:**
- [x] `crates/backup-manager/src/lib.rs` - Backup crate
- [x] `crates/backup-manager/src/backup.rs` - Backup creation and listing
- [x] `crates/backup-manager/src/restore.rs` - Restore implementation
- [x] `crates/backup-manager/src/scheduler.rs` - Scheduled backups

### 3.2 What to Backup
```
backup-{timestamp}.tar.gz:
├── config/
│   ├── system/general.json
│   ├── system/mqtt_subscriptions.cfg
│   └── plugins/
├── data/
│   ├── system/plugindatabase.json
│   └── plugins/
└── metadata.json (version, timestamp, etc.)
```

### 3.3 API Endpoints
```
POST   /api/backup/create          - Create backup      ✅
GET    /api/backup                 - List backups        ✅
GET    /api/backup/:name/download  - Download backup     ✅
DELETE /api/backup/:name           - Delete backup       ✅
POST   /api/backup/restore/:id     - Restore from backup (future)
POST   /api/backup/schedule        - Scheduled backups   (future)
```

### 3.4 Web UI
- [x] Backup page in web UI (`/backup`)
- [x] One-click backup creation (HTMX)
- [x] Backup download (link to API)
- [x] Restore with confirmation dialog (HTMX hx-confirm)
- [x] Schedule configuration (`GET/PUT /api/backup/schedule` + backup page UI)

## 4. Polish Existing Features (Priority: MEDIUM)

### 4.1 Error Handling Improvements
- [x] Better error messages in web UI (validation errors surfaced inline)
- [x] Detailed error logging (tracing in all key paths)
- [x] Recovery suggestions (error messages include actionable hints)
- [x] Validation before operations (miniserver + MQTT forms validated pre-save)

### 4.2 Configuration Validation
**Files created:**
- [x] `crates/loxberry-config/src/validation.rs` - Validation with unit tests

```rust
pub fn validate_miniserver_config(config: &MiniserverConfig) -> Result<()>  // ✅
pub fn validate_mqtt_config(config: &MqttConfig) -> Result<()>              // ✅
```

### 4.3 Web UI Improvements
- [x] Better form validation (HTML5 pattern/type/min/max on all forms)
- [x] Loading states (htmx-indicator spinner in style.css)
- [x] Success/error notifications (toast system via htmx:afterSwap)
- [x] Confirmation dialogs for destructive actions (hx-confirm on delete/restore)
- [x] Better mobile responsiveness (responsive nav at ≤768px)

### 4.4 API Improvements
- [x] Rate limiting (tower_governor: 1 req/s burst 10 per IP)
- [x] Request validation (validate_miniserver_config / validate_mqtt_config before save)
- [x] Better status codes (400 validation, 404 not found, 429 rate limit, 500 internal)
- [x] OpenAPI/Swagger documentation (`/api-docs` page with all endpoints grouped)

### 4.5 Performance Optimizations
- [ ] Database connection pooling (deferred to Phase 6 — no DB yet)
- [ ] Caching for frequently accessed data (deferred to Phase 6)
- [ ] Lazy loading of plugin list (deferred to Phase 6)
- [x] Optimize config file reads (Arc<RwLock<>> shared config, single read per request)

## 5. Documentation (Priority: LOW)

### 5.1 User Documentation
- [x] Installation guide (`INSTALL.md` — Docker quickstart, local dev, env vars)
- [x] Configuration guide (`INSTALL.md` — key config options table)
- [x] Plugin installation guide (`INSTALL.md` — plugin ZIP workflow)
- [x] Troubleshooting guide (`INSTALL.md` — common errors and fixes)

### 5.2 Developer Documentation
- [x] API documentation (`/api-docs` web page + CLAUDE.md API patterns)
- [x] Plugin development guide (`CLAUDE.md` — hook types, env vars, plugin.cfg format)
- [x] Contributing guide (`CONTRIBUTING.md` already present)
- [x] Architecture documentation (`CLAUDE.md` — crate dependency graph, workspace structure)

## Implementation Order

### Week 1: Logging & SDK Foundation
1. ✅ Setup project structure
2. ✅ Create logging crate with rotation
3. ✅ Copy SDK libraries to Docker image
4. ✅ Implement environment variable injection

### Week 2: SDK Integration & Testing
1. ✅ Create plugin executor wrapper (Perl/PHP/Bash)
2. [ ] Test with real Perl plugins (requires Docker runtime — Phase 6)
3. [ ] Verify all SDK paths work (requires Docker runtime — Phase 6)
4. [ ] Fix any compatibility issues (requires Docker runtime — Phase 6)

### Week 3: Backup & Restore
1. ✅ Create backup-manager crate
2. ✅ Implement backup creation
3. ✅ Implement restore functionality
4. ✅ Add backup API endpoints
5. ✅ Create backup UI page

### Week 4: Polish & Optimize
1. ✅ Improve error handling throughout (inline validation errors + toast notifications)
2. ✅ Add validation (config validation module)
3. ✅ Optimize performance (Arc<RwLock<>> config sharing)
4. ✅ Add documentation (INSTALL.md + /api-docs)
5. ✅ Final testing (cargo test passes, 5 unit tests in validation.rs)

## Success Criteria

- [ ] At least 3 real LoxBerry plugins can be installed and run (deferred to Phase 6 — requires Docker runtime testing)
- [x] Logs are properly rotated and accessible via UI (`/logs` viewer with search)
- [x] Backups can be created, downloaded, and restored (`/backup` UI + REST API)
- [x] Config validation wired into Miniserver add/edit and MQTT config forms
- [x] Log level adjustable at runtime via `GET/PUT /api/system/log-level`
- [x] All forms have proper validation (HTML5 + server-side validation)
- [x] Error messages are clear and actionable (inline errors + toast notifications)
- [ ] CI/CD pipeline passes (GitHub Actions — requires push to main)
- [ ] Docker images build for all platforms (requires CI/CD)
