# Phase 6 Plan: System Updates, Monitoring & Production Features

<div align="center">

![Status](https://img.shields.io/badge/Status-Planning-yellow)
![Phase](https://img.shields.io/badge/Phase-6-blue)
![Priority](https://img.shields.io/badge/Priority-Production%20Ready-red)

</div>

## Overview

Phase 6 focuses on production-readiness features:
- System update mechanism
- Advanced monitoring and observability
- Email notifications
- Scheduled tasks and cron jobs
- Network services integration
- Health checks and diagnostics

## 1. System Update Mechanism (Priority: HIGH)

### 1.1 Update Manager Crate

**Files to create:**
- `crates/update-manager/src/lib.rs` - New crate
- `crates/update-manager/src/github.rs` - GitHub release API client
- `crates/update-manager/src/installer.rs` - Update installer
- `crates/update-manager/src/rollback.rs` - Rollback mechanism

### 1.2 Update Workflow

```rust
pub struct UpdateManager {
    current_version: Version,
    update_channel: Channel,  // Stable, Beta, Nightly
}

pub enum Channel {
    Stable,      // Production releases
    Beta,        // Testing releases
    Nightly,     // Development builds
}

impl UpdateManager {
    // Check for available updates
    pub async fn check_for_updates(&self) -> Result<Option<Release>>

    // Download update package
    pub async fn download_update(&self, release: &Release) -> Result<PathBuf>

    // Verify signature
    pub async fn verify_signature(&self, package: &Path) -> Result<bool>

    // Apply update (docker pull or binary replace)
    pub async fn apply_update(&self, package: &Path) -> Result<()>

    // Rollback to previous version
    pub async fn rollback(&self) -> Result<()>
}
```

### 1.3 Update Strategies

#### Docker-based Updates
```bash
# Pull latest image
docker pull ghcr.io/boernmaster/rustylox:latest

# Graceful container restart
docker compose down && docker compose up -d
```

#### Binary Updates (for non-Docker installs)
```bash
# Download new binary
curl -L https://github.com/boernmaster/RustyLox/releases/download/v1.1.0/loxberry-daemon

# Backup current binary
cp /usr/local/bin/loxberry-daemon /usr/local/bin/loxberry-daemon.backup

# Replace binary
mv loxberry-daemon /usr/local/bin/loxberry-daemon

# Restart service
systemctl restart loxberry
```

### 1.4 Update Configuration

**File**: `config/system/updates.json`
```json
{
  "channel": "stable",
  "auto_check": true,
  "check_interval_hours": 24,
  "auto_install": false,
  "backup_before_update": true,
  "rollback_enabled": true
}
```

### 1.5 API Endpoints

```
GET  /api/updates/check           - Check for available updates
GET  /api/updates/current          - Get current version info
POST /api/updates/download/:version - Download update package
POST /api/updates/install/:version  - Install update
POST /api/updates/rollback         - Rollback to previous version
GET  /api/updates/changelog/:version - Get changelog
```

### 1.6 Web UI

**Page**: `/settings/updates`

Features:
- Display current version
- Display available updates
- Update channel selection (Stable/Beta/Nightly)
- One-click update button
- Update history
- Rollback option
- Changelog viewer
- Backup before update option

### 1.7 Safety Features

- ✅ Automatic backup before update
- ✅ Signature verification (GPG)
- ✅ Rollback capability
- ✅ Config migration scripts
- ✅ Database backup
- ✅ Plugin compatibility check
- ✅ Update notification (email/push)

## 2. Advanced Monitoring & Observability (Priority: HIGH)

### 2.1 Metrics Collection

**Files to create:**
- `crates/metrics/src/lib.rs` - Metrics collection
- `crates/metrics/src/prometheus.rs` - Prometheus exporter
- `crates/metrics/src/health.rs` - Health checks

### 2.2 Metrics to Track

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

### 2.3 Prometheus Integration

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

### 2.4 Health Checks

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

### 2.5 Alerting System

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

## 3. Email Notifications (Priority: MEDIUM)

### 3.1 Email Manager Crate

**Files to create:**
- `crates/email-manager/src/lib.rs` - Email client
- `crates/email-manager/src/smtp.rs` - SMTP client
- `crates/email-manager/src/templates.rs` - Email templates

### 3.2 Email Configuration

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

### 3.3 Email Types

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

### 3.4 Email Templates

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

### 3.5 API Endpoints

```
POST /api/email/send            - Send email
POST /api/email/test            - Test email configuration
GET  /api/email/config          - Get email config
PUT  /api/email/config          - Update email config
GET  /api/email/history         - Get sent email history
```

## 4. Scheduled Tasks & Cron Jobs (Priority: MEDIUM)

### 4.1 Task Scheduler Crate

**Files to create:**
- `crates/task-scheduler/src/lib.rs` - Task scheduler
- `crates/task-scheduler/src/cron.rs` - Cron expression parser
- `crates/task-scheduler/src/executor.rs` - Task executor

### 4.2 Built-in Scheduled Tasks

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

### 4.3 Task Configuration

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

### 4.4 Web UI

**Page**: `/settings/tasks`

Features:
- List all scheduled tasks
- Add custom tasks
- Enable/disable tasks
- Edit cron schedule
- View task execution history
- Manual task execution

### 4.5 Task Execution Logging

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

## 5. Network Services Integration (Priority: LOW)

### 5.1 Time Server (NTP)

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

### 5.2 DNS Management

**Features**:
- Configure DNS servers
- MDNS/Avahi support (loxberry.local)
- DNS cache monitoring

### 5.3 Network Diagnostics

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

## 6. Advanced Diagnostics (Priority: MEDIUM)

### 6.1 System Information

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

### 6.2 Log Viewer

**Page**: `/logs`

Features:
- View system logs (filterable)
- View plugin logs
- Log level filtering
- Search logs
- Download logs
- Real-time log streaming (SSE)

### 6.3 Performance Profiling

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

### Week 1: Update System
1. Create update-manager crate
2. Implement GitHub release API client
3. Implement Docker update strategy
4. Add update UI page
5. Test update workflow

### Week 2: Monitoring & Metrics
1. Create metrics crate
2. Implement Prometheus exporter
3. Add health checks
4. Create alerting system
5. Add metrics dashboard

### Week 3: Email & Notifications
1. Create email-manager crate
2. Implement SMTP client
3. Create email templates
4. Add notification preferences
5. Test email delivery

### Week 4: Scheduled Tasks
1. Create task-scheduler crate
2. Implement cron parser
3. Add built-in tasks
4. Create task management UI
5. Test scheduled execution

### Week 5: Network Services & Diagnostics
1. Implement network diagnostics
2. Add time server configuration
3. Create system info page
4. Add log viewer
5. Performance profiling

### Week 6: Polish & Testing
1. Integration testing
2. Performance optimization
3. Documentation
4. Security audit
5. Production deployment guide

## Success Criteria

- [ ] Updates can be checked and installed from UI
- [ ] Rollback works correctly
- [ ] Prometheus metrics are exported
- [ ] Health checks detect issues
- [ ] Email notifications are sent correctly
- [ ] Scheduled tasks execute on time
- [ ] Task execution is logged
- [ ] Network diagnostics work
- [ ] System info is accurate
- [ ] Logs are viewable and searchable
- [ ] All features are documented

## Dependencies

```toml
[dependencies]
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

## Security Considerations

1. **Update Security**:
   - Signature verification (GPG)
   - HTTPS for downloads
   - Checksum verification

2. **Email Security**:
   - Encrypted password storage
   - TLS/SSL for SMTP
   - Rate limiting

3. **API Security**:
   - Authentication for sensitive endpoints
   - Rate limiting
   - Input validation

4. **Metrics Security**:
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
