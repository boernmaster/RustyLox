# Phase 5+ Implementation Plan

<div align="center">

![Status](https://img.shields.io/badge/Status-Planning-yellow)
![Phase](https://img.shields.io/badge/Phase-5%2B-blue)
![Priority](https://img.shields.io/badge/Priority-SDK%20%26%20Logging-red)

</div>

## Overview
Implement SDK compatibility, logging, backup/restore, and polish existing features.

## 1. Logging Framework (Priority: HIGH)

### 1.1 Structured Logging System
- [x] Basic tracing-subscriber (already implemented)
- [ ] Log rotation with tracing-appender
- [ ] Per-component log levels
- [ ] Plugin-specific log files
- [ ] Web UI log viewer

**Files to create:**
- `crates/loxberry-logging/src/lib.rs` - New crate
- `crates/loxberry-logging/src/rotation.rs` - Log rotation
- `crates/loxberry-logging/src/plugin_logger.rs` - Plugin-specific logging

### 1.2 Log Management
- [ ] Log level configuration via API
- [ ] Download logs via web UI
- [ ] Log search and filtering
- [ ] Log retention policies

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
- [ ] Update Dockerfile to copy SDK libraries
- [ ] Create directory structure in container
- [ ] Set correct permissions (loxberry:loxberry)

### 2.2 Environment Variable Injection
**Files to create/modify:**
- `crates/plugin-manager/src/environment.rs` - Environment builder

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
**Files to create:**
- `crates/plugin-manager/src/executor.rs` - Script executor with environment

```rust
pub struct PluginExecutor {
    pub async fn execute_perl(&self, script: &Path, plugin: &PluginEntry) -> Result<Output>
    pub async fn execute_php(&self, script: &Path, plugin: &PluginEntry) -> Result<Output>
    pub async fn execute_bash(&self, script: &Path, plugin: &PluginEntry) -> Result<Output>
}
```

## 3. Backup & Restore (Priority: MEDIUM)

### 3.1 Backup System
**Files to create:**
- `crates/backup-manager/src/lib.rs` - New crate
- `crates/backup-manager/src/backup.rs` - Backup implementation
- `crates/backup-manager/src/restore.rs` - Restore implementation
- `crates/backup-manager/src/scheduler.rs` - Scheduled backups

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
POST   /api/backup/create          - Create backup
GET    /api/backup/list            - List backups
GET    /api/backup/download/:id    - Download backup file
POST   /api/backup/restore/:id     - Restore from backup
DELETE /api/backup/:id             - Delete backup
POST   /api/backup/schedule        - Configure scheduled backups
```

### 3.4 Web UI
- [ ] Backup page in web UI
- [ ] One-click backup creation
- [ ] Backup download
- [ ] Restore with confirmation
- [ ] Schedule configuration

## 4. Polish Existing Features (Priority: MEDIUM)

### 4.1 Error Handling Improvements
- [ ] Better error messages in web UI
- [ ] Detailed error logging
- [ ] Recovery suggestions
- [ ] Validation before operations

### 4.2 Configuration Validation
**Files to modify:**
- `crates/loxberry-config/src/validation.rs` - New file

```rust
pub fn validate_miniserver_config(config: &MiniserverConfig) -> Result<()>
pub fn validate_mqtt_config(config: &MqttConfig) -> Result<()>
pub fn validate_plugin_config(config: &PluginConfig) -> Result<()>
```

### 4.3 Web UI Improvements
- [ ] Better form validation
- [ ] Loading states
- [ ] Success/error notifications
- [ ] Confirmation dialogs for destructive actions
- [ ] Better mobile responsiveness

### 4.4 API Improvements
- [ ] Rate limiting
- [ ] Request validation
- [ ] Better status codes
- [ ] OpenAPI/Swagger documentation

### 4.5 Performance Optimizations
- [ ] Database connection pooling (if we add database)
- [ ] Caching for frequently accessed data
- [ ] Lazy loading of plugin list
- [ ] Optimize config file reads

## 5. Documentation (Priority: LOW)

### 5.1 User Documentation
- [ ] Installation guide
- [ ] Configuration guide
- [ ] Plugin installation guide
- [ ] Troubleshooting guide

### 5.2 Developer Documentation
- [ ] API documentation
- [ ] Plugin development guide
- [ ] Contributing guide
- [ ] Architecture documentation

## Implementation Order

### Week 1: Logging & SDK Foundation
1. ✅ Setup project structure
2. Create logging crate with rotation
3. Copy SDK libraries to Docker image
4. Implement environment variable injection

### Week 2: SDK Integration & Testing
1. Create plugin executor wrapper
2. Test with real Perl plugins
3. Verify all SDK paths work
4. Fix any compatibility issues

### Week 3: Backup & Restore
1. Create backup-manager crate
2. Implement backup creation
3. Implement restore functionality
4. Add backup API endpoints
5. Create backup UI page

### Week 4: Polish & Optimize
1. Improve error handling throughout
2. Add validation to all forms
3. Optimize performance
4. Add documentation
5. Final testing

## Success Criteria

- [ ] At least 3 real LoxBerry plugins can be installed and run
- [ ] Logs are properly rotated and accessible via UI
- [ ] Backups can be created and restored successfully
- [ ] All forms have proper validation
- [ ] Error messages are clear and actionable
- [ ] CI/CD pipeline passes
- [ ] Docker images build for all platforms
