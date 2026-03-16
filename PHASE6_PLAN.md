# Phase 6 Plan: System Updates, Monitoring & Production Features

<div align="center">

![Status](https://img.shields.io/badge/Status-Planning-yellow)
![Phase](https://img.shields.io/badge/Phase-6-blue)
![Priority](https://img.shields.io/badge/Priority-Production%20Ready-red)

</div>

## Overview

Phase 6 focuses on production-readiness features and SDK validation:
- Performance optimizations (deferred from Phase 5)
- SDK integration testing with real plugins (deferred from Phase 5)
- Advanced monitoring and observability
- Email notifications
- Scheduled tasks and cron jobs
- Network services integration
- Health checks and diagnostics

**Note**: RustyLox updates are delivered via Docker images. No update management UI is required - users simply pull new images and restart containers.

## 1. Performance Optimizations (Priority: HIGH - Deferred from Phase 5)

### 1.1 Database Connection Pooling

**Note**: Deferred from Phase 5 as no database is implemented yet. Will be addressed when database layer is added.

**Future Implementation**:
```rust
// When PostgreSQL/SQLite is added
pub struct DbPool {
    pool: deadpool_postgres::Pool,
    max_connections: usize,
}
```

### 1.2 Caching for Frequently Accessed Data

**Files to modify:**
- `crates/plugin-manager/src/lib.rs` - Add caching layer
- `crates/loxberry-config/src/lib.rs` - Add config caching
- `crates/mqtt-gateway/src/lib.rs` - Cache MQTT subscriptions

**Implementation**:
```rust
use std::sync::Arc;
use dashmap::DashMap;

pub struct CacheManager {
    plugin_cache: Arc<DashMap<String, PluginEntry>>,
    config_cache: Arc<DashMap<String, serde_json::Value>>,
    ttl_seconds: u64,
}

impl CacheManager {
    pub fn get_plugin(&self, name: &str) -> Option<PluginEntry> {
        self.plugin_cache.get(name).map(|r| r.clone())
    }

    pub fn invalidate(&self, key: &str) {
        self.plugin_cache.remove(key);
    }
}
```

**Caching Strategy**:
- Plugin list: Cache for 60 seconds (invalidate on install/uninstall)
- Config files: Cache for 30 seconds (invalidate on write)
- MQTT subscriptions: Cache indefinitely (invalidate on config change)

### 1.3 Lazy Loading of Plugin List

**Files to modify:**
- `web-ui/src/handlers/plugins.rs` - Implement pagination
- `web-ui/templates/plugins.html` - Add HTMX infinite scroll

**Implementation**:
```rust
pub async fn list_plugins(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Json<PluginListResponse> {
    let offset = params.page * params.page_size;
    let limit = params.page_size;

    let plugins = state.plugin_manager
        .list_plugins_paginated(offset, limit)
        .await?;

    Json(PluginListResponse {
        plugins,
        has_more: plugins.len() == params.page_size as usize,
    })
}
```

**HTMX Template**:
```html
<div hx-get="/api/plugins?page=1"
     hx-trigger="revealed"
     hx-swap="afterend">
    <!-- Plugin cards loaded dynamically -->
</div>
```

## 2. Plugin Execution & SDK Testing (Priority: HIGH - Deferred from Phase 5)

### 2.1 Plugin Daemon Management

**Objective**: Run plugin background processes and manage their lifecycle

**Files to modify:**
- `crates/plugin-manager/src/daemon.rs` - Daemon lifecycle management
- `crates/plugin-manager/src/lib.rs` - Add daemon control

**Implementation**:
```rust
pub struct PluginDaemon {
    plugin_name: String,
    process: Option<Child>,
    pid_file: PathBuf,
    log_file: PathBuf,
}

impl PluginDaemon {
    // Start daemon process
    pub async fn start(&mut self) -> Result<()> {
        let daemon_script = format!("{}/daemon/daemon.pl", self.plugin_dir());

        let child = Command::new("perl")
            .arg(&daemon_script)
            .env("LBHOMEDIR", "/opt/loxberry")
            .env("LBPPLUGINDIR", &self.plugin_name)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Write PID file
        self.write_pid(child.id())?;
        self.process = Some(child);

        Ok(())
    }

    // Stop daemon process
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            child.kill().await?;
            self.remove_pid()?;
        }
        Ok(())
    }

    // Check if daemon is running
    pub fn is_running(&self) -> bool {
        if let Some(pid) = self.read_pid() {
            // Check if process exists
            std::fs::metadata(format!("/proc/{}", pid)).is_ok()
        } else {
            false
        }
    }

    // Restart daemon
    pub async fn restart(&mut self) -> Result<()> {
        self.stop().await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.start().await
    }
}
```

**API Endpoints**:
```
POST /api/plugins/:name/daemon/start   - Start plugin daemon
POST /api/plugins/:name/daemon/stop    - Stop plugin daemon
POST /api/plugins/:name/daemon/restart - Restart plugin daemon
GET  /api/plugins/:name/daemon/status  - Get daemon status
GET  /api/plugins/:name/daemon/logs    - Get daemon logs
```

**Web UI Updates** (`/plugins` page):
- Add daemon status indicator (running/stopped)
- Add start/stop/restart buttons
- Show daemon logs in modal
- Auto-refresh daemon status
- **Add "Open Web Interface" button** (links to `/plugins/:name/` or `/admin/plugins/:name/`)
  - Button only visible if plugin has web interface
  - Opens in new tab/window
  - Icon indicator showing public vs authenticated interface
- Show web interface availability status (has web interface or not)
- Display web interface URL in plugin details

**Example Plugin Card**:
```html
<div class="plugin-card">
  <h3>{{ plugin.title }}</h3>
  <p>Version: {{ plugin.version }}</p>

  <!-- Daemon status -->
  <div class="daemon-status">
    <span class="status-{{ daemon_status }}">
      Daemon: {{ daemon_status }}
    </span>
    <button hx-post="/api/plugins/{{ plugin.name }}/daemon/start">Start</button>
    <button hx-post="/api/plugins/{{ plugin.name }}/daemon/stop">Stop</button>
  </div>

  <!-- Web interface link -->
  {% if plugin.has_web_interface() %}
  <div class="web-interface">
    <a href="{{ plugin.web_interface_url() }}" target="_blank" class="btn">
      🌐 Open Web Interface
    </a>
  </div>
  {% endif %}

  <!-- Other actions -->
  <button hx-delete="/api/plugins/{{ plugin.name }}">Uninstall</button>
</div>
```

### 2.2 Plugin Web Interface Directory Structure

**Objective**: Create and manage plugin web interface directories

**Plugin Archive Structure** (in ZIP file):
```
plugin-name/
├── plugin.cfg                 # Plugin metadata
├── webfrontend/
│   ├── html/                 # Public web files (copied to /opt/loxberry/webfrontend/html/plugins/<name>/)
│   │   ├── index.html
│   │   ├── styles.css
│   │   └── scripts.js
│   └── htmlauth/             # Authenticated web files (copied to /opt/loxberry/webfrontend/htmlauth/plugins/<name>/)
│       ├── index.php
│       ├── settings.php
│       └── admin.php
├── daemon/
│   └── daemon.pl
└── data/
    └── ...
```

**Installed Directory Structure**:
```
/opt/loxberry/
├── webfrontend/
│   ├── html/
│   │   └── plugins/
│   │       └── <plugin-name>/        # Public web files (no auth required)
│   │           ├── index.html
│   │           ├── styles.css
│   │           └── scripts.js
│   └── htmlauth/
│       └── plugins/
│           └── <plugin-name>/        # Authenticated web files (login required)
│               ├── index.php
│               ├── settings.php
│               └── admin.php
```

**Files to modify:**
- `crates/plugin-manager/src/installer.rs` - Create web directories during install
- `crates/plugin-manager/src/lib.rs` - Add web directory management

**Implementation**:
```rust
impl PluginInstaller {
    async fn create_web_directories(&self, plugin_name: &str) -> Result<()> {
        let base = "/opt/loxberry/webfrontend";

        // Create public HTML directory
        let html_dir = format!("{}/html/plugins/{}", base, plugin_name);
        tokio::fs::create_dir_all(&html_dir).await?;

        // Create authenticated HTML directory
        let htmlauth_dir = format!("{}/htmlauth/plugins/{}", base, plugin_name);
        tokio::fs::create_dir_all(&htmlauth_dir).await?;

        // Set permissions (readable by web server)
        set_permissions(&html_dir, 0o755)?;
        set_permissions(&htmlauth_dir, 0o755)?;

        Ok(())
    }

    async fn copy_web_files(&self, plugin: &PluginEntry) -> Result<()> {
        let plugin_dir = format!("/opt/loxberry/data/plugins/{}", plugin.name);

        // Copy from plugin archive to web directories
        if let Some(html_files) = plugin.web_files("html") {
            self.copy_directory(
                &format!("{}/webfrontend/html", plugin_dir),
                &format!("/opt/loxberry/webfrontend/html/plugins/{}", plugin.name)
            ).await?;
        }

        if let Some(htmlauth_files) = plugin.web_files("htmlauth") {
            self.copy_directory(
                &format!("{}/webfrontend/htmlauth", plugin_dir),
                &format!("/opt/loxberry/webfrontend/htmlauth/plugins/{}", plugin.name)
            ).await?;
        }

        Ok(())
    }

    async fn cleanup_web_directories(&self, plugin_name: &str) -> Result<()> {
        // Remove on uninstall
        let html_dir = format!("/opt/loxberry/webfrontend/html/plugins/{}", plugin_name);
        let htmlauth_dir = format!("/opt/loxberry/webfrontend/htmlauth/plugins/{}", plugin_name);

        tokio::fs::remove_dir_all(&html_dir).await.ok();
        tokio::fs::remove_dir_all(&htmlauth_dir).await.ok();

        Ok(())
    }
}
```

**Detecting Plugin Web Interfaces**:
```rust
impl PluginEntry {
    // Check if plugin has web interface
    pub fn has_web_interface(&self) -> bool {
        let html_dir = format!("/opt/loxberry/webfrontend/html/plugins/{}", self.name);
        let htmlauth_dir = format!("/opt/loxberry/webfrontend/htmlauth/plugins/{}", self.name);

        PathBuf::from(&html_dir).exists() || PathBuf::from(&htmlauth_dir).exists()
    }

    // Get web interface URL
    pub fn web_interface_url(&self) -> Option<String> {
        if self.has_web_interface() {
            // Prefer authenticated interface if available
            let htmlauth_dir = format!("/opt/loxberry/webfrontend/htmlauth/plugins/{}", self.name);
            if PathBuf::from(&htmlauth_dir).exists() {
                return Some(format!("/admin/plugins/{}/", self.name));
            }
            Some(format!("/plugins/{}/", self.name))
        } else {
            None
        }
    }
}
```

**Environment Variables for Plugins**:
```bash
LBPHTMLDIR=/opt/loxberry/webfrontend/html/plugins/<plugin-name>
LBPHTMLAUTHDIR=/opt/loxberry/webfrontend/htmlauth/plugins/<plugin-name>
```

### 2.3 Plugin Web Interface Serving

**Objective**: Serve plugin HTML/PHP pages through RustyLox web server

**Files to modify:**
- `crates/web-ui/src/lib.rs` - Add plugin routes
- `crates/web-ui/src/handlers/plugin_web.rs` - Plugin web handler (new file)

**Implementation**:
```rust
// Serve public plugin files (no authentication)
pub async fn serve_plugin_public(
    Path((plugin_name, path)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Response> {
    let plugin_dir = format!("/opt/loxberry/webfrontend/html/plugins/{}", plugin_name);
    serve_plugin_file(&plugin_dir, &path, false).await
}

// Serve authenticated plugin files (requires login)
pub async fn serve_plugin_auth(
    Path((plugin_name, path)): Path<(String, String)>,
    State(state): State<AppState>,
    // TODO: Add authentication extension when auth is implemented
) -> Result<Response> {
    let plugin_dir = format!("/opt/loxberry/webfrontend/htmlauth/plugins/{}", plugin_name);
    serve_plugin_file(&plugin_dir, &path, true).await
}

async fn serve_plugin_file(
    base_dir: &str,
    path: &str,
    require_auth: bool,
) -> Result<Response> {
    // Default to index.html if path is empty or ends with /
    let file_path = if path.is_empty() || path.ends_with('/') {
        PathBuf::from(base_dir).join("index.html")
    } else {
        PathBuf::from(base_dir).join(path)
    };

    // Security: ensure path is within plugin directory
    let canonical = file_path.canonicalize()
        .map_err(|_| Error::NotFound)?;

    if !canonical.starts_with(base_dir) {
        return Err(Error::Unauthorized);
    }

    // Check file extension
    match file_path.extension().and_then(|s| s.to_str()) {
        Some("php") => serve_php(&file_path).await,
        Some("html") | Some("htm") => serve_static_html(&file_path).await,
        Some("css") => serve_static_file(&file_path, "text/css").await,
        Some("js") => serve_static_file(&file_path, "application/javascript").await,
        Some("png") => serve_static_file(&file_path, "image/png").await,
        Some("jpg") | Some("jpeg") => serve_static_file(&file_path, "image/jpeg").await,
        Some("svg") => serve_static_file(&file_path, "image/svg+xml").await,
        _ => Err(Error::UnsupportedFileType),
    }
}

// Execute PHP scripts
async fn serve_php(file_path: &Path) -> Result<Response> {
    let output = Command::new("php-cgi")  // Use php-cgi for web execution
        .arg(file_path)
        .env("LBHOMEDIR", "/opt/loxberry")
        .env("REDIRECT_STATUS", "200")  // Required for php-cgi
        .output()
        .await?;

    if !output.status.success() {
        return Err(Error::PhpExecutionFailed(
            String::from_utf8_lossy(&output.stderr).to_string()
        ));
    }

    // Parse CGI headers and body
    let response_text = String::from_utf8(output.stdout)?;
    parse_cgi_response(&response_text)
}

async fn serve_static_html(file_path: &Path) -> Result<Response> {
    let content = tokio::fs::read_to_string(file_path).await?;
    Ok(Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(content))?)
}

async fn serve_static_file(file_path: &Path, content_type: &str) -> Result<Response> {
    let content = tokio::fs::read(file_path).await?;
    Ok(Response::builder()
        .header("Content-Type", content_type)
        .body(Body::from(content))?)
}

fn parse_cgi_response(cgi_output: &str) -> Result<Response> {
    // Parse CGI headers (Content-Type, etc.) and body
    let mut headers = vec![];
    let mut body_start = 0;

    for (i, line) in cgi_output.lines().enumerate() {
        if line.is_empty() {
            body_start = i + 1;
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.push((key.trim(), value.trim()));
        }
    }

    let body = cgi_output.lines().skip(body_start).collect::<Vec<_>>().join("\n");

    let mut response = Response::builder();
    for (key, value) in headers {
        response = response.header(key, value);
    }

    Ok(response.body(Body::from(body))?)
}
```

**Routes**:
```rust
Router::new()
    // Public plugin web interface (no auth)
    .route("/plugins/:name/*path", get(serve_plugin_public))

    // Authenticated plugin web interface
    .route("/admin/plugins/:name/*path", get(serve_plugin_auth))
```

**Docker Updates**:

Update `Dockerfile` to include PHP-CGI:
```dockerfile
RUN apt-get update && apt-get install -y \
    perl \
    php-cli \
    php-cgi \          # Add PHP-CGI for web execution
    bash \
    ca-certificates
```

**Security**:
- Path traversal protection (canonicalize paths)
- Authentication/authorization for `/admin/plugins/*` routes
- Rate limiting on PHP execution
- Input sanitization for PHP scripts
- File type whitelist (only serve safe file types - html, css, js, php, images)
- Disable dangerous PHP functions in php.ini (exec, system, shell_exec, etc.)
- Sandboxed PHP execution (disable file operations outside plugin directory)
- CSRF protection for plugin forms
- XSS protection (Content-Security-Policy headers)

### 2.4 Plugin Script Execution

**Objective**: Execute plugin custom scripts and scheduled tasks

**Files to modify:**
- `crates/plugin-manager/src/executor.rs` - Script execution engine

**Implementation**:
```rust
pub struct PluginScriptExecutor {
    plugin_name: String,
    base_dir: PathBuf,
}

impl PluginScriptExecutor {
    // Execute arbitrary plugin script
    pub async fn execute_script(
        &self,
        script_path: &str,
        args: Vec<String>,
    ) -> Result<ScriptOutput> {
        let full_path = self.base_dir.join(script_path);

        // Detect script type
        let interpreter = self.detect_interpreter(&full_path)?;

        let output = Command::new(&interpreter)
            .arg(&full_path)
            .args(&args)
            .envs(self.build_env())
            .output()
            .await?;

        Ok(ScriptOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    fn detect_interpreter(&self, path: &Path) -> Result<String> {
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match ext {
            "pl" => Ok("perl".to_string()),
            "php" => Ok("php".to_string()),
            "sh" => Ok("bash".to_string()),
            "py" => Ok("python3".to_string()),
            _ => Err(Error::UnsupportedScriptType),
        }
    }

    fn build_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("LBHOMEDIR".to_string(), "/opt/loxberry".to_string());
        env.insert("LBPPLUGINDIR".to_string(), self.plugin_name.clone());
        env.insert("LBPDATADIR".to_string(),
            format!("/opt/loxberry/data/plugins/{}", self.plugin_name));
        env.insert("LBPLOGDIR".to_string(),
            format!("/opt/loxberry/log/plugins/{}", self.plugin_name));
        env.insert("LBPCONFIGDIR".to_string(),
            format!("/opt/loxberry/config/plugins/{}", self.plugin_name));
        env
    }
}
```

**API Endpoints**:
```
POST /api/plugins/:name/execute  - Execute plugin script
GET  /api/plugins/:name/scripts  - List available scripts
```

### 2.5 Plugin Resource Access & Monitoring

**Objective**: Monitor plugin resource usage and file access

**Implementation**:
```rust
pub struct PluginResourceMonitor {
    plugin_name: String,
}

impl PluginResourceMonitor {
    // Get plugin disk usage
    pub async fn get_disk_usage(&self) -> Result<DiskUsage> {
        let data_dir = format!("/opt/loxberry/data/plugins/{}", self.plugin_name);
        let log_dir = format!("/opt/loxberry/log/plugins/{}", self.plugin_name);
        let config_dir = format!("/opt/loxberry/config/plugins/{}", self.plugin_name);

        Ok(DiskUsage {
            data_size: dir_size(&data_dir).await?,
            log_size: dir_size(&log_dir).await?,
            config_size: dir_size(&config_dir).await?,
            total: /* sum */,
        })
    }

    // Get plugin process info (if daemon running)
    pub async fn get_process_info(&self) -> Result<Option<ProcessInfo>> {
        let pid_file = format!("/opt/loxberry/log/plugins/{}/daemon.pid", self.plugin_name);

        if let Some(pid) = read_pid(&pid_file) {
            Ok(Some(ProcessInfo {
                pid,
                cpu_percent: get_cpu_usage(pid)?,
                memory_bytes: get_memory_usage(pid)?,
                uptime: get_uptime(pid)?,
            }))
        } else {
            Ok(None)
        }
    }
}
```

**API Endpoints**:
```
GET /api/plugins/:name/resources  - Get resource usage
GET /api/plugins/:name/processes  - Get running processes
```

### 2.6 Test with Real Perl Plugins

**Objective**: Validate SDK compatibility with existing LoxBerry Perl plugins

**Test Plugins**:
1. **Simple Plugin**: Basic Perl script using LoxBerry::System
2. **MQTT Plugin**: Uses LoxBerry::IO::MQTT
3. **HTTP Plugin**: Uses LoxBerry::Web
4. **Logger Plugin**: Uses LoxBerry::Log

**Test Environment** (Docker):
```bash
# Create test volumes
mkdir -p volumes/test/plugins/{plugin1,plugin2}

# Mount test plugins
docker compose -f docker-compose.test.yml up -d
```

**Test Cases**:
```perl
# Test 1: LoxBerry::System
use LoxBerry::System;
my $lbhomedir = $ENV{LBHOMEDIR};
print "Home: $lbhomedir\n";

# Test 2: Plugin paths
use LoxBerry::System;
my $plugindir = lbplugindir();
print "Plugin dir: $plugindir\n";

# Test 3: Config reading
use LoxBerry::System;
my $config = LoxBerry::System::read_config("myconfig.cfg");

# Test 4: MQTT publish
use LoxBerry::IO::MQTT;
my $mqtt = LoxBerry::IO::MQTT->new();
$mqtt->publish("test/topic", "Hello");
```

### 2.7 Verify All SDK Paths Work

**SDK Paths to Verify**:
```bash
# Perl SDK
LBHOMEDIR=/opt/loxberry
LBSCONFIG=/opt/loxberry/config/system
LBSDATA=/opt/loxberry/data/system
LBSLOG=/opt/loxberry/log/system
LBPPLUGINDIR=<plugin-name>
LBPHTMLDIR=/opt/loxberry/webfrontend/html/plugins/<plugin>
LBPHTMLAUTHDIR=/opt/loxberry/webfrontend/htmlauth/plugins/<plugin>
LBPDATADIR=/opt/loxberry/data/plugins/<plugin>
LBPLOGDIR=/opt/loxberry/log/plugins/<plugin>
LBPCONFIGDIR=/opt/loxberry/config/plugins/<plugin>
```

**Verification Script** (`tests/sdk_paths_test.sh`):
```bash
#!/bin/bash
set -e

# Test all paths exist
test -d $LBHOMEDIR || exit 1
test -d $LBSCONFIG || exit 1
test -d $LBSDATA || exit 1
test -d $LBSLOG || exit 1

# Test plugin paths
test -d $LBPDATADIR || exit 1
test -d $LBPLOGDIR || exit 1
test -d $LBPCONFIGDIR || exit 1

echo "All SDK paths verified successfully"
```

### 2.8 Fix Any Compatibility Issues

**Known Issues to Address**:

1. **Path Separators**: Windows vs Unix
   - Solution: Use PathBuf consistently in Rust

2. **File Permissions**: Plugin files may need specific permissions
   - Solution: Set correct permissions during install

3. **Environment Variables**: Missing or incorrect env vars
   - Solution: Validate env in plugin executor

4. **Perl Module Availability**: Some CPAN modules may be missing
   - Solution: Update Dockerfile with required Perl modules

**Compatibility Testing Checklist**:
- [ ] Perl plugins can read/write config files
- [ ] Perl plugins can access SDK functions
- [ ] Perl plugins can send MQTT messages
- [ ] Perl plugins can write logs
- [ ] PHP plugins work (if any)
- [ ] Bash plugins work (if any)
- [ ] Plugin web interfaces load correctly
- [ ] Plugin daemons start/stop correctly

## 3. Docker Updates (Reference Only)

**RustyLox updates are delivered via Docker images. No UI management required.**

### Update Process (User Manual)

```bash
# Check current version
docker exec rustylox cat /opt/loxberry/VERSION

# Pull latest image
docker pull ghcr.io/boernmaster/rustylox:latest

# Backup config (optional)
tar -czf rustylox-backup-$(date +%Y%m%d).tar.gz volumes/

# Graceful restart
docker compose down
docker compose up -d

# Verify new version
docker exec rustylox cat /opt/loxberry/VERSION
```

### Available Tags

- `ghcr.io/boernmaster/rustylox:latest` - Latest stable release
- `ghcr.io/boernmaster/rustylox:v1.2.0` - Specific version
- `ghcr.io/boernmaster/rustylox:v1.2` - Minor version
- `ghcr.io/boernmaster/rustylox:v1` - Major version

### Rollback

```bash
# Use specific version tag
docker compose down
docker pull ghcr.io/boernmaster/rustylox:v1.1.0
# Update docker-compose.yml to pin version
docker compose up -d
```

## 4. Advanced Monitoring & Observability (Priority: HIGH)

### 4.1 Metrics Collection

**Files to create:**
- `crates/metrics/src/lib.rs` - Metrics collection
- `crates/metrics/src/prometheus.rs` - Prometheus exporter
- `crates/metrics/src/health.rs` - Health checks

### 4.2 Metrics to Track

```rust
pub struct SystemMetrics {
    // System metrics
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub memory_total: u64,
    pub disk_usage: u64,
    pub disk_total: u64,
    pub uptime: Duration,

    // Application metrics
    pub active_connections: usize,
    pub mqtt_messages_received: u64,
    pub mqtt_messages_sent: u64,
    pub http_requests_total: u64,
    pub http_requests_errors: u64,
    pub plugin_count: usize,
    pub miniserver_count: usize,

    // Performance metrics
    pub request_duration_ms: Vec<f64>,
    pub mqtt_latency_ms: f64,
    pub miniserver_latency_ms: HashMap<String, f64>,
}
```

### 4.3 Prometheus Integration

**Endpoint**: `/metrics`

Export Prometheus-compatible metrics:
```
# HELP loxberry_uptime_seconds System uptime in seconds
# TYPE loxberry_uptime_seconds counter
loxberry_uptime_seconds 3600

# HELP loxberry_mqtt_messages_total Total MQTT messages processed
# TYPE loxberry_mqtt_messages_total counter
loxberry_mqtt_messages_total{direction="received"} 1234
loxberry_mqtt_messages_total{direction="sent"} 567

# HELP loxberry_http_request_duration_seconds HTTP request duration
# TYPE loxberry_http_request_duration_seconds histogram
loxberry_http_request_duration_seconds_bucket{le="0.005"} 123
loxberry_http_request_duration_seconds_bucket{le="0.01"} 456
```

### 4.4 Health Checks

**Enhanced** `/health` endpoint:

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime": "3h 24m",
  "checks": {
    "database": "ok",
    "mqtt_broker": "ok",
    "miniserver_1": "ok",
    "disk_space": "ok",
    "memory": "ok"
  },
  "metrics": {
    "cpu_usage": 15.3,
    "memory_usage": 234567890,
    "mqtt_connected": true,
    "active_plugins": 5
  }
}
```

### 4.5 Alerting System

**Files to create:**
- `crates/alerting/src/lib.rs` - Alert manager
- `crates/alerting/src/rules.rs` - Alert rules
- `crates/alerting/src/channels.rs` - Notification channels

**Alert Rules** (`config/system/alerts.json`):
```json
{
  "rules": [
    {
      "name": "High CPU Usage",
      "condition": "cpu_usage > 90",
      "duration": "5m",
      "severity": "warning",
      "actions": ["email", "log"]
    },
    {
      "name": "Miniserver Disconnected",
      "condition": "miniserver_connected == false",
      "duration": "1m",
      "severity": "critical",
      "actions": ["email", "push", "log"]
    },
    {
      "name": "Disk Space Low",
      "condition": "disk_usage_percent > 90",
      "duration": "10m",
      "severity": "warning",
      "actions": ["email"]
    }
  ]
}
```

## 5. Email Notifications (Priority: MEDIUM)

### 5.1 Email Manager Crate

**Files to create:**
- `crates/email-manager/src/lib.rs` - Email client
- `crates/email-manager/src/smtp.rs` - SMTP client
- `crates/email-manager/src/templates.rs` - Email templates

### 5.2 Email Configuration

**File**: `config/system/email.json`
```json
{
  "enabled": true,
  "smtp_host": "smtp.gmail.com",
  "smtp_port": 587,
  "smtp_user": "user@gmail.com",
  "smtp_pass": "encrypted_password",
  "smtp_tls": true,
  "from_address": "loxberry@example.com",
  "from_name": "LoxBerry System",
  "notification_addresses": ["admin@example.com"]
}
```

### 5.3 Email Types

```rust
pub enum EmailType {
    SystemUpdate {
        from_version: String,
        to_version: String,
        success: bool,
    },
    Alert {
        severity: Severity,
        title: String,
        message: String,
    },
    PluginInstalled {
        name: String,
        version: String,
    },
    BackupCompleted {
        size: u64,
        duration: Duration,
    },
    MiniserverDisconnected {
        miniserver_name: String,
        timestamp: DateTime<Utc>,
    },
}
```

### 5.4 Email Templates

**HTML Email Template**:
```html
<!DOCTYPE html>
<html>
<head>
    <style>
        body { font-family: Arial, sans-serif; }
        .header { background: #ff6b35; color: white; padding: 20px; }
        .content { padding: 20px; }
        .footer { background: #f5f5f5; padding: 10px; text-align: center; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🏠 RustyLox Notification</h1>
    </div>
    <div class="content">
        <h2>{{ title }}</h2>
        <p>{{ message }}</p>
        <p><strong>Time:</strong> {{ timestamp }}</p>
    </div>
    <div class="footer">
        <p>RustyLox v{{ version }} | <a href="http://your-loxberry-url">Dashboard</a></p>
    </div>
</body>
</html>
```

### 5.5 API Endpoints

```
POST /api/email/send            - Send email
POST /api/email/test            - Test email configuration
GET  /api/email/config          - Get email config
PUT  /api/email/config          - Update email config
GET  /api/email/history         - Get sent email history
```

## 6. Scheduled Tasks & Cron Jobs (Priority: MEDIUM)

### 6.1 Task Scheduler Crate

**Files to create:**
- `crates/task-scheduler/src/lib.rs` - Task scheduler
- `crates/task-scheduler/src/cron.rs` - Cron expression parser
- `crates/task-scheduler/src/executor.rs` - Task executor

### 6.2 Built-in Scheduled Tasks

```rust
pub struct ScheduledTask {
    name: String,
    schedule: CronExpression,  // "0 0 * * *" = daily at midnight
    task_type: TaskType,
    enabled: bool,
}

pub enum TaskType {
    Backup,                    // Daily backup
    UpdateCheck,               // Check for updates
    LogRotation,               // Rotate logs
    PluginMaintenance,         // Plugin cleanup
    MetricsAggregation,        // Aggregate metrics
    HealthCheck,               // System health check
    Custom(String),            // User-defined script
}
```

### 6.3 Task Configuration

**File**: `config/system/scheduled_tasks.json`
```json
{
  "tasks": [
    {
      "name": "Daily Backup",
      "schedule": "0 2 * * *",
      "type": "backup",
      "enabled": true
    },
    {
      "name": "Update Check",
      "schedule": "0 */6 * * *",
      "type": "update_check",
      "enabled": true
    },
    {
      "name": "Log Rotation",
      "schedule": "0 0 * * 0",
      "type": "log_rotation",
      "enabled": true
    }
  ]
}
```

### 6.4 Web UI

**Page**: `/settings/tasks`

Features:
- List all scheduled tasks
- Add custom tasks
- Enable/disable tasks
- Edit cron schedule
- View task execution history
- Manual task execution

### 6.5 Task Execution Logging

```rust
pub struct TaskExecution {
    task_name: String,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    status: ExecutionStatus,
    output: String,
    error: Option<String>,
}

pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

## 7. Network Services Integration (Priority: LOW)

### 7.1 Time Server (NTP)

**Features**:
- Configure NTP servers
- Timezone management
- Auto-sync on boot
- Manual time sync button

**Configuration**:
```json
{
  "ntp_servers": [
    "pool.ntp.org",
    "time.google.com"
  ],
  "timezone": "Europe/Vienna",
  "auto_sync": true
}
```

### 7.2 DNS Management

**Features**:
- Configure DNS servers
- MDNS/Avahi support (loxberry.local)
- DNS cache monitoring

### 7.3 Network Diagnostics

**Tools**:
- Ping test
- Port scanner
- Network interface info
- Connection test to Miniserver
- MQTT broker connectivity test

**API Endpoints**:
```
POST /api/network/ping              - Ping host
POST /api/network/portscan          - Scan ports
GET  /api/network/interfaces        - List network interfaces
POST /api/network/test/miniserver   - Test Miniserver connection
POST /api/network/test/mqtt         - Test MQTT connection
```

## 8. Advanced Diagnostics (Priority: MEDIUM)

### 8.1 System Information

```rust
pub struct SystemInfo {
    // Hardware
    cpu_model: String,
    cpu_cores: usize,
    memory_total: u64,
    disk_total: u64,

    // Software
    os_name: String,
    os_version: String,
    kernel_version: String,
    rust_version: String,

    // Network
    hostname: String,
    ip_addresses: Vec<IpAddr>,
    mac_addresses: Vec<String>,

    // Docker (if running in container)
    docker_version: Option<String>,
    container_id: Option<String>,
}
```

### 8.2 Log Viewer

**Page**: `/logs`

Features:
- View system logs (filterable)
- View plugin logs
- Log level filtering
- Search logs
- Download logs
- Real-time log streaming (SSE)

### 8.3 Performance Profiling

**Features**:
- CPU profiling
- Memory profiling
- Request tracing
- Slow query detection
- Performance bottleneck identification

**Tools**:
- Integration with tokio-console
- Flame graph generation
- Request timeline

## Implementation Timeline

### Week 1-2: Performance Optimizations (Phase 5 Deferrals)
1. Implement caching layer for plugins and config
2. Add pagination/lazy loading for plugin list
3. Cache invalidation strategies
4. Performance testing and benchmarking

### Week 3-4: Plugin Execution & SDK Testing (Phase 5 Deferrals)
1. Implement plugin web directory creation during install
2. Implement plugin daemon management (start/stop/restart)
3. Implement plugin web interface serving (HTML/PHP/static files)
4. Add PHP-CGI support to Docker image
5. Implement plugin script execution engine
6. Implement plugin resource monitoring
7. Set up Docker test environment for plugins
8. Test real Perl/PHP/Bash plugins in Docker
9. Verify all SDK paths and environment variables (including web paths)
10. Fix compatibility issues

### Week 5: Monitoring & Metrics
1. Create metrics crate
2. Implement Prometheus exporter
3. Add health checks
4. Create alerting system
5. Add metrics dashboard

### Week 6: Email & Notifications
1. Create email-manager crate
2. Implement SMTP client
3. Create email templates
4. Add notification preferences
5. Test email delivery

### Week 7: Scheduled Tasks
1. Create task-scheduler crate
2. Implement cron parser
3. Add built-in tasks
4. Create task management UI
5. Test scheduled execution

### Week 8: Network Services & Diagnostics
1. Implement network diagnostics
2. Add time server configuration
3. Create system info page
4. Add log viewer
5. Performance profiling

### Week 9: Polish & Testing
1. Integration testing
2. Performance optimization
3. Documentation
4. Security audit
5. Production deployment guide

## Success Criteria

### Performance Optimizations (Phase 5 Deferrals)
- [ ] Plugin list caching is implemented
- [ ] Config file caching is implemented
- [ ] Plugin list pagination/lazy loading works
- [ ] Database connection pooling ready for future implementation

### Plugin Execution & SDK (Phase 5 Deferrals)
- [ ] Plugin web directories created during installation (html/htmlauth)
- [ ] Plugin web files copied to correct directories
- [ ] Plugin daemon start/stop/restart works
- [ ] Plugin daemon status monitoring implemented
- [ ] Public plugin web interfaces served at `/plugins/:name/`
- [ ] Authenticated plugin web interfaces served at `/admin/plugins/:name/`
- [ ] PHP-CGI execution works correctly
- [ ] Static files (CSS/JS/images) served from plugin directories
- [ ] Plugin custom scripts can be executed
- [ ] Plugin resource monitoring (disk, CPU, memory) works
- [ ] Real Perl/PHP plugins tested in Docker environment
- [ ] All SDK paths verified and working (including LBPHTMLDIR, LBPHTMLAUTHDIR)
- [ ] Plugin compatibility issues resolved

### Monitoring & Observability
- [ ] Prometheus metrics are exported
- [ ] Health checks detect issues correctly
- [ ] Alert rules trigger appropriately

### Email & Notifications
- [ ] Email notifications are sent correctly
- [ ] SMTP configuration works with TLS
- [ ] Email templates render properly

### Scheduled Tasks
- [ ] Scheduled tasks execute on time
- [ ] Task execution is logged
- [ ] Cron expressions parsed correctly

### Network & Diagnostics
- [ ] Network diagnostics work
- [ ] System info is accurate
- [ ] Logs are viewable and searchable

### General
- [ ] All features are documented
- [ ] Docker deployment works smoothly

## Dependencies

```toml
[dependencies]
# Caching (for performance optimizations)
dashmap = "5.5"  # Already in use
moka = "0.12"    # Optional: advanced caching with TTL

# Process management (for plugin daemons & web execution)
tokio-process = "0.2"  # Part of tokio, already in use
sysinfo = "0.30"       # For process monitoring

# HTTP responses (for serving plugin files)
http = "0.2"           # For building HTTP responses
mime_guess = "2.0"     # For determining MIME types

# Email
lettre = "0.11"
lettre_email = "0.9"

# Metrics
prometheus = "0.13"
metrics = "0.21"
metrics-exporter-prometheus = "0.12"

# Task scheduling
cron = "0.12"
tokio-cron-scheduler = "0.9"

# System info
sysinfo = "0.30"
hostname = "0.3"

# Network
trust-dns-resolver = "0.23"
ping = "0.5"
```

### System Dependencies (Docker/apt)

```dockerfile
# Required packages for plugin execution
RUN apt-get update && apt-get install -y \
    perl \              # Perl interpreter for plugins
    libperl-dev \       # Perl development libraries
    php-cli \           # PHP command-line interpreter
    php-cgi \           # PHP-CGI for web execution (REQUIRED for plugin web interfaces)
    php-mbstring \      # PHP multibyte string support
    php-curl \          # PHP cURL support
    php-json \          # PHP JSON support
    bash \              # Bash shell for scripts
    ca-certificates \   # SSL certificates
    procps \            # Process utilities (ps, top, etc.)
    && rm -rf /var/lib/apt/lists/*
```

## Security Considerations

1. **Plugin Execution Security**:
   - Path traversal protection for plugin files
   - Sandbox plugin processes (consider user namespaces)
   - Resource limits (CPU, memory, disk) per plugin
   - Input sanitization for script execution
   - Limit allowed interpreters (perl, php, bash only)
   - No arbitrary command execution
   - Log all plugin script executions

2. **Docker Security**:
   - Pull images from trusted registry (ghcr.io)
   - Verify image signatures
   - Use specific version tags for production

3. **Email Security**:
   - Encrypted password storage
   - TLS/SSL for SMTP
   - Rate limiting

4. **API Security**:
   - Authentication for sensitive endpoints
   - Rate limiting (especially for plugin execution)
   - Input validation
   - Plugin execution authorization (only allow plugin owners)
   - Audit logging for all plugin operations

5. **Metrics Security**:
   - Sensitive data filtering
   - Access control for metrics endpoint
   - Audit logging

## Next Steps After Phase 6

Phase 7 will focus on:
- Production hardening
- Security enhancements
- Performance optimization
- High availability
- Multi-instance support
- Cloud deployment options
