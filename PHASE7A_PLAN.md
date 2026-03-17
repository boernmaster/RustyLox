# Phase 7a Plan: Complete Web UI for Backend Features

<div align="center">

![Status](https://img.shields.io/badge/Status-Planning-yellow)
![Phase](https://img.shields.io/badge/Phase-7a-blue)
![Priority](https://img.shields.io/badge/Priority-High-orange)

</div>

## Overview

Phase 7a focuses on building comprehensive web UI for all backend features implemented in Phase 6 and Phase 7. While the backend APIs are complete, many features lack user-facing interfaces.

**Goal**: Complete, polished web interfaces for authentication, monitoring, email, tasks, diagnostics, and all other backend functionality.

**Technology**: Askama templates + HTMX + CSS (continuing existing web-ui architecture)

---

## Current State

### ✅ Existing UI Pages
- `/` - Dashboard
- `/miniservers` - Miniserver management
- `/mqtt` - MQTT monitor and configuration
- `/plugins` - Plugin management
- `/settings` - Basic system settings

### ❌ Missing UI for Backend Features
- Authentication (login, user management, API keys)
- Email configuration and test
- Task scheduler management
- Network diagnostics tools
- System health dashboard
- Backup/restore interface
- Audit log viewer
- Database management
- Security settings
- User profile management

---

## 1. Authentication & User Management UI

### 1.1 Login Page (`/login`)

**Template**: `web-ui/templates/login.html`

**Features**:
- Clean, centered login form
- Username and password fields
- "Remember me" checkbox
- Password visibility toggle
- Error messages (failed login, account locked)
- Link to password reset (future)
- Branding (RustyLox logo)

**Template Structure**:
```html
<!DOCTYPE html>
<html>
<head>
    <title>Login - RustyLox</title>
    <link rel="stylesheet" href="/static/css/auth.css">
</head>
<body class="auth-page">
    <div class="auth-container">
        <img src="/static/logo.svg" alt="RustyLox" class="auth-logo">
        <h1>Welcome to RustyLox</h1>

        {% if error %}
        <div class="alert alert-error">{{ error }}</div>
        {% endif %}

        <form hx-post="/api/auth/login" hx-target="body" hx-swap="outerHTML">
            <div class="form-group">
                <label>Username</label>
                <input type="text" name="username" required autofocus>
            </div>

            <div class="form-group">
                <label>Password</label>
                <input type="password" name="password" required>
                <button type="button" class="password-toggle">Show</button>
            </div>

            <div class="form-group">
                <label>
                    <input type="checkbox" name="remember_me">
                    Remember me
                </label>
            </div>

            <button type="submit" class="btn btn-primary btn-block">
                Login
            </button>
        </form>

        <div class="auth-footer">
            <small>RustyLox v{{ version }}</small>
        </div>
    </div>
</body>
</html>
```

**Handler** (`web-ui/src/handlers/auth.rs`):
```rust
pub async fn show_login(
    State(state): State<AppState>,
    Query(params): Query<LoginParams>,
) -> Html<String> {
    let template = LoginTemplate {
        error: params.error,
        version: env!("CARGO_PKG_VERSION"),
    };
    Html(template.render().unwrap_or_default())
}

pub async fn handle_login(
    State(state): State<AppState>,
    Form(credentials): Form<LoginForm>,
) -> Result<Redirect> {
    // Call auth API
    let response = state.auth_service
        .authenticate(&credentials.username, &credentials.password)
        .await?;

    // Set cookie with JWT token
    Ok(Redirect::to("/dashboard"))
}
```

### 1.2 User Management Page (`/admin/users`)

**Features**:
- List all users with roles
- Search and filter users
- Create new user button (opens modal)
- Edit user (roles, permissions)
- Delete user (with confirmation)
- User status indicators (active, locked)
- Last login timestamp
- Password reset button

**Template Structure**:
```html
<div class="page-header">
    <h1>User Management</h1>
    <button hx-get="/admin/users/new"
            hx-target="#user-modal"
            hx-swap="innerHTML"
            class="btn btn-primary">
        + New User
    </button>
</div>

<div class="search-bar">
    <input type="text"
           hx-get="/api/users"
           hx-trigger="keyup changed delay:300ms"
           hx-target="#users-list"
           placeholder="Search users...">
</div>

<table id="users-list">
    <thead>
        <tr>
            <th>Username</th>
            <th>Email</th>
            <th>Roles</th>
            <th>Status</th>
            <th>Last Login</th>
            <th>Actions</th>
        </tr>
    </thead>
    <tbody>
        {% for user in users %}
        <tr>
            <td>{{ user.username }}</td>
            <td>{{ user.email }}</td>
            <td>
                {% for role in user.roles %}
                <span class="badge">{{ role }}</span>
                {% endfor %}
            </td>
            <td>
                {% if user.locked %}
                <span class="status status-locked">Locked</span>
                {% else %}
                <span class="status status-active">Active</span>
                {% endif %}
            </td>
            <td>{{ user.last_login | format_timestamp }}</td>
            <td>
                <button hx-get="/admin/users/{{ user.id }}/edit"
                        hx-target="#user-modal"
                        class="btn btn-sm">Edit</button>
                <button hx-delete="/api/users/{{ user.id }}"
                        hx-confirm="Delete user {{ user.username }}?"
                        hx-target="closest tr"
                        hx-swap="outerHTML swap:1s"
                        class="btn btn-sm btn-danger">Delete</button>
            </td>
        </tr>
        {% endfor %}
    </tbody>
</table>

<!-- Modal for create/edit user -->
<div id="user-modal" class="modal"></div>
```

**Create User Modal**:
```html
<div class="modal-content">
    <h2>Create New User</h2>
    <form hx-post="/api/users"
          hx-target="#users-list"
          hx-swap="beforeend">
        <div class="form-group">
            <label>Username</label>
            <input type="text" name="username" required>
        </div>

        <div class="form-group">
            <label>Email</label>
            <input type="email" name="email" required>
        </div>

        <div class="form-group">
            <label>Password</label>
            <input type="password" name="password" required>
        </div>

        <div class="form-group">
            <label>Roles</label>
            <select name="roles" multiple>
                <option value="admin">Admin</option>
                <option value="operator">Operator</option>
                <option value="viewer">Viewer</option>
                <option value="plugin_manager">Plugin Manager</option>
            </select>
        </div>

        <div class="modal-actions">
            <button type="submit" class="btn btn-primary">Create User</button>
            <button type="button" class="btn" onclick="closeModal()">Cancel</button>
        </div>
    </form>
</div>
```

### 1.3 API Key Management (`/admin/api-keys`)

**Features**:
- List all API keys
- Key name, prefix (lbx_xxx), permissions
- Created date, last used date
- Expiry date (if set)
- Create new API key (shows full key once)
- Revoke/delete keys
- Copy key to clipboard

**Template**:
```html
<div class="page-header">
    <h1>API Keys</h1>
    <button hx-post="/api/auth/keys/new"
            hx-target="#key-modal"
            class="btn btn-primary">
        + Generate API Key
    </button>
</div>

<table>
    <thead>
        <tr>
            <th>Name</th>
            <th>Key</th>
            <th>Permissions</th>
            <th>Created</th>
            <th>Last Used</th>
            <th>Expires</th>
            <th>Actions</th>
        </tr>
    </thead>
    <tbody>
        {% for key in api_keys %}
        <tr>
            <td>{{ key.name }}</td>
            <td><code>{{ key.prefix }}***</code></td>
            <td>
                {% for perm in key.permissions %}
                <span class="badge">{{ perm }}</span>
                {% endfor %}
            </td>
            <td>{{ key.created_at | format_date }}</td>
            <td>{{ key.last_used_at | format_date_or_never }}</td>
            <td>
                {% if key.expires_at %}
                {{ key.expires_at | format_date }}
                {% else %}
                Never
                {% endif %}
            </td>
            <td>
                <button hx-delete="/api/auth/keys/{{ key.id }}"
                        hx-confirm="Revoke this API key?"
                        class="btn btn-sm btn-danger">Revoke</button>
            </td>
        </tr>
        {% endfor %}
    </tbody>
</table>

<!-- Modal shows full key after creation -->
<div id="key-modal"></div>
```

**New Key Modal** (shown only once after creation):
```html
<div class="modal-content">
    <h2>API Key Created</h2>
    <div class="alert alert-warning">
        <strong>Important:</strong> Copy this key now. You won't be able to see it again.
    </div>

    <div class="key-display">
        <code id="api-key">{{ api_key }}</code>
        <button onclick="copyToClipboard('api-key')" class="btn btn-sm">
            📋 Copy
        </button>
    </div>

    <div class="key-details">
        <p><strong>Name:</strong> {{ name }}</p>
        <p><strong>Permissions:</strong> {{ permissions | join(", ") }}</p>
    </div>

    <button class="btn btn-primary" onclick="closeModal()">
        I've saved the key
    </button>
</div>
```

### 1.4 User Profile Page (`/profile`)

**Features**:
- View current user info
- Change password
- Update email
- View sessions
- Logout from all devices
- Delete account (with confirmation)

---

## 2. System Health Dashboard (`/health` or `/dashboard/health`)

**Features**:
- Real-time system metrics
- CPU usage gauge
- Memory usage gauge
- Disk usage gauge
- Service status indicators (MQTT, Miniserver, Database)
- Uptime display
- Active connections count
- Recent alerts/warnings
- Historical graphs (last 24h)

**Template Structure**:
```html
<div class="dashboard-grid">
    <!-- System Metrics -->
    <div class="metric-card">
        <h3>CPU Usage</h3>
        <div class="gauge" data-value="{{ cpu_usage }}">
            <span class="gauge-value">{{ cpu_usage }}%</span>
        </div>
    </div>

    <div class="metric-card">
        <h3>Memory</h3>
        <div class="gauge" data-value="{{ memory_percent }}">
            <span class="gauge-value">{{ memory_used }} / {{ memory_total }}</span>
        </div>
    </div>

    <div class="metric-card">
        <h3>Disk Space</h3>
        <div class="gauge" data-value="{{ disk_percent }}">
            <span class="gauge-value">{{ disk_used }} / {{ disk_total }}</span>
        </div>
    </div>

    <div class="metric-card">
        <h3>Uptime</h3>
        <div class="uptime">
            {{ uptime }}
        </div>
    </div>

    <!-- Service Status -->
    <div class="services-card">
        <h3>Services</h3>
        <div class="service-list">
            <div class="service {% if mqtt_connected %}status-ok{% else %}status-error{% endif %}">
                <span class="service-name">MQTT Broker</span>
                <span class="service-status">{{ mqtt_status }}</span>
            </div>

            <div class="service {% if miniserver_connected %}status-ok{% else %}status-error{% endif %}">
                <span class="service-name">Miniserver</span>
                <span class="service-status">{{ miniserver_status }}</span>
            </div>

            <div class="service status-ok">
                <span class="service-name">Database</span>
                <span class="service-status">Connected</span>
            </div>
        </div>
    </div>

    <!-- Recent Alerts -->
    <div class="alerts-card">
        <h3>Recent Alerts</h3>
        <div class="alert-list">
            {% for alert in recent_alerts %}
            <div class="alert alert-{{ alert.severity }}">
                <span class="alert-time">{{ alert.timestamp | format_time }}</span>
                <span class="alert-message">{{ alert.message }}</span>
            </div>
            {% endfor %}
        </div>
    </div>

    <!-- Performance Graph -->
    <div class="graph-card">
        <h3>CPU & Memory (Last 24h)</h3>
        <canvas id="performance-chart"></canvas>
    </div>
</div>

<!-- Auto-refresh every 5 seconds -->
<script>
    setInterval(() => {
        htmx.trigger('#health-dashboard', 'refresh');
    }, 5000);
</script>
```

**Handler**:
```rust
pub async fn health_dashboard(
    State(state): State<AppState>,
) -> Html<String> {
    let metrics = state.metrics_collector.get_current_metrics().await?;
    let alerts = state.alert_manager.get_recent_alerts(10).await?;

    let template = HealthDashboardTemplate {
        cpu_usage: metrics.cpu_percent,
        memory_used: format_bytes(metrics.memory_used),
        memory_total: format_bytes(metrics.memory_total),
        memory_percent: (metrics.memory_used * 100 / metrics.memory_total),
        disk_used: format_bytes(metrics.disk_used),
        disk_total: format_bytes(metrics.disk_total),
        disk_percent: (metrics.disk_used * 100 / metrics.disk_total),
        uptime: format_duration(metrics.uptime),
        mqtt_connected: state.mqtt_gateway.is_connected(),
        miniserver_connected: state.miniserver_client.is_connected(),
        recent_alerts: alerts,
    };

    Html(template.render().unwrap())
}
```

---

## 3. Email Configuration UI (`/settings/email`)

**Features**:
- SMTP server configuration form
- Test email button
- Email template preview
- Notification preferences
- Email history/logs

**Template**:
```html
<div class="settings-page">
    <h1>Email Configuration</h1>

    <form hx-put="/api/email/config"
          hx-target="#email-status"
          class="settings-form">
        <div class="form-group">
            <label>SMTP Server</label>
            <input type="text" name="smtp_host" value="{{ config.smtp_host }}" required>
        </div>

        <div class="form-group">
            <label>Port</label>
            <input type="number" name="smtp_port" value="{{ config.smtp_port }}" required>
        </div>

        <div class="form-group">
            <label>Username</label>
            <input type="text" name="smtp_user" value="{{ config.smtp_user }}">
        </div>

        <div class="form-group">
            <label>Password</label>
            <input type="password" name="smtp_pass" value="{{ config.smtp_pass }}">
        </div>

        <div class="form-group">
            <label>
                <input type="checkbox" name="smtp_tls" {% if config.smtp_tls %}checked{% endif %}>
                Use TLS/SSL
            </label>
        </div>

        <div class="form-group">
            <label>From Address</label>
            <input type="email" name="from_address" value="{{ config.from_address }}" required>
        </div>

        <div class="form-group">
            <label>From Name</label>
            <input type="text" name="from_name" value="{{ config.from_name }}">
        </div>

        <div class="form-group">
            <label>Notification Recipients</label>
            <textarea name="notification_addresses" rows="3">{{ config.notification_addresses | join("\n") }}</textarea>
            <small>One email per line</small>
        </div>

        <div class="form-actions">
            <button type="submit" class="btn btn-primary">Save Configuration</button>
            <button type="button"
                    hx-post="/api/email/test"
                    hx-include="[name='smtp_host'], [name='smtp_port'], [name='smtp_user'], [name='smtp_pass']"
                    hx-target="#email-status"
                    class="btn btn-secondary">
                📧 Send Test Email
            </button>
        </div>
    </form>

    <div id="email-status"></div>

    <div class="email-history">
        <h2>Recent Emails</h2>
        <table>
            <thead>
                <tr>
                    <th>Date</th>
                    <th>To</th>
                    <th>Subject</th>
                    <th>Status</th>
                </tr>
            </thead>
            <tbody>
                {% for email in email_history %}
                <tr>
                    <td>{{ email.sent_at | format_datetime }}</td>
                    <td>{{ email.to }}</td>
                    <td>{{ email.subject }}</td>
                    <td>
                        {% if email.success %}
                        <span class="status status-ok">Sent</span>
                        {% else %}
                        <span class="status status-error">Failed</span>
                        {% endif %}
                    </td>
                </tr>
                {% endfor %}
            </tbody>
        </table>
    </div>
</div>
```

---

## 4. Task Scheduler UI (`/settings/tasks`)

**Features**:
- List all scheduled tasks
- Enable/disable tasks
- Edit cron schedule
- Add custom tasks
- View execution history
- Manual task execution
- Task logs viewer

**Template**:
```html
<div class="page-header">
    <h1>Scheduled Tasks</h1>
    <button hx-get="/settings/tasks/new"
            hx-target="#task-modal"
            class="btn btn-primary">
        + New Task
    </button>
</div>

<table class="tasks-table">
    <thead>
        <tr>
            <th>Name</th>
            <th>Schedule</th>
            <th>Type</th>
            <th>Status</th>
            <th>Last Run</th>
            <th>Next Run</th>
            <th>Actions</th>
        </tr>
    </thead>
    <tbody>
        {% for task in tasks %}
        <tr>
            <td>{{ task.name }}</td>
            <td><code>{{ task.schedule }}</code></td>
            <td><span class="badge">{{ task.task_type }}</span></td>
            <td>
                <label class="toggle">
                    <input type="checkbox"
                           {% if task.enabled %}checked{% endif %}
                           hx-put="/api/tasks/{{ task.id }}/toggle"
                           hx-target="closest tr">
                    <span class="toggle-slider"></span>
                </label>
            </td>
            <td>
                {% if task.last_run %}
                {{ task.last_run | format_datetime }}
                {% if task.last_run_success %}
                <span class="status-ok">✓</span>
                {% else %}
                <span class="status-error">✗</span>
                {% endif %}
                {% else %}
                Never
                {% endif %}
            </td>
            <td>{{ task.next_run | format_datetime }}</td>
            <td>
                <button hx-post="/api/tasks/{{ task.id }}/execute"
                        class="btn btn-sm">▶ Run Now</button>
                <button hx-get="/settings/tasks/{{ task.id }}/history"
                        hx-target="#task-history-modal"
                        class="btn btn-sm">📊 History</button>
                <button hx-get="/settings/tasks/{{ task.id }}/edit"
                        hx-target="#task-modal"
                        class="btn btn-sm">✏ Edit</button>
            </td>
        </tr>
        {% endfor %}
    </tbody>
</table>

<div id="task-modal" class="modal"></div>
<div id="task-history-modal" class="modal"></div>
```

**Add/Edit Task Modal**:
```html
<div class="modal-content">
    <h2>{% if task %}Edit Task{% else %}New Task{% endif %}</h2>

    <form hx-post="/api/tasks" hx-target="#tasks-table">
        <div class="form-group">
            <label>Task Name</label>
            <input type="text" name="name" value="{{ task.name }}" required>
        </div>

        <div class="form-group">
            <label>Schedule (Cron Expression)</label>
            <input type="text" name="schedule" value="{{ task.schedule }}" required>
            <small>Examples:
                <code>0 2 * * *</code> (daily at 2 AM),
                <code>*/15 * * * *</code> (every 15 minutes)
            </small>
        </div>

        <div class="form-group">
            <label>Task Type</label>
            <select name="task_type" required>
                <option value="backup">Backup</option>
                <option value="log_rotation">Log Rotation</option>
                <option value="health_check">Health Check</option>
                <option value="plugin_update">Plugin Update Check</option>
                <option value="custom">Custom Script</option>
            </select>
        </div>

        <div class="form-group" id="custom-script" style="display: none;">
            <label>Script Path</label>
            <input type="text" name="script_path">
        </div>

        <div class="form-group">
            <label>
                <input type="checkbox" name="enabled" checked>
                Enabled
            </label>
        </div>

        <div class="modal-actions">
            <button type="submit" class="btn btn-primary">Save Task</button>
            <button type="button" class="btn" onclick="closeModal()">Cancel</button>
        </div>
    </form>
</div>
```

---

## 5. Network Diagnostics UI (`/diagnostics/network`)

**Features**:
- Ping tool
- Port scanner
- Connection tests (Miniserver, MQTT, Internet)
- Network interface info
- DNS lookup tool
- Traceroute
- Results display

**Template**:
```html
<div class="diagnostics-page">
    <h1>Network Diagnostics</h1>

    <div class="diagnostics-grid">
        <!-- Ping Tool -->
        <div class="tool-card">
            <h2>Ping</h2>
            <form hx-post="/api/network/ping" hx-target="#ping-results">
                <input type="text" name="host" placeholder="Enter hostname or IP" required>
                <button type="submit" class="btn">Ping</button>
            </form>
            <div id="ping-results" class="results"></div>
        </div>

        <!-- Port Scanner -->
        <div class="tool-card">
            <h2>Port Scanner</h2>
            <form hx-post="/api/network/portscan" hx-target="#portscan-results">
                <input type="text" name="host" placeholder="Host" required>
                <input type="text" name="ports" placeholder="Ports (e.g., 80,443,1883)" required>
                <button type="submit" class="btn">Scan</button>
            </form>
            <div id="portscan-results" class="results"></div>
        </div>

        <!-- Connectivity Tests -->
        <div class="tool-card">
            <h2>Quick Tests</h2>
            <button hx-post="/api/network/test/miniserver"
                    hx-target="#miniserver-test"
                    class="btn btn-block">
                Test Miniserver Connection
            </button>
            <div id="miniserver-test" class="results"></div>

            <button hx-post="/api/network/test/mqtt"
                    hx-target="#mqtt-test"
                    class="btn btn-block">
                Test MQTT Broker
            </button>
            <div id="mqtt-test" class="results"></div>

            <button hx-post="/api/network/test/internet"
                    hx-target="#internet-test"
                    class="btn btn-block">
                Test Internet Connectivity
            </button>
            <div id="internet-test" class="results"></div>
        </div>

        <!-- Network Interfaces -->
        <div class="tool-card">
            <h2>Network Interfaces</h2>
            <button hx-get="/api/network/interfaces"
                    hx-target="#interfaces-results"
                    class="btn">
                Show Interfaces
            </button>
            <div id="interfaces-results" class="results"></div>
        </div>
    </div>
</div>
```

**Results Template** (ping example):
```html
<div class="ping-result">
    <div class="result-header">
        <strong>Ping {{ host }}</strong>
        {% if success %}
        <span class="status status-ok">Success</span>
        {% else %}
        <span class="status status-error">Failed</span>
        {% endif %}
    </div>

    {% if success %}
    <div class="result-details">
        <p><strong>Latency:</strong> {{ latency_ms }}ms</p>
        <p><strong>Packet Loss:</strong> {{ packet_loss }}%</p>
        <p><strong>TTL:</strong> {{ ttl }}</p>
    </div>
    {% else %}
    <div class="result-error">
        <p>{{ error_message }}</p>
    </div>
    {% endif %}
</div>
```

---

## 6. Backup & Restore UI (`/settings/backup`)

**Features**:
- Create backup button
- List existing backups
- Download backup file
- Delete old backups
- Restore from backup
- Backup size and contents
- Scheduled backup configuration

**Template**:
```html
<div class="backup-page">
    <div class="page-header">
        <h1>Backup & Restore</h1>
        <button hx-post="/api/backup/create"
                hx-target="#backup-status"
                hx-indicator="#backup-spinner"
                class="btn btn-primary">
            📦 Create Backup Now
        </button>
    </div>

    <div id="backup-status"></div>
    <div id="backup-spinner" class="htmx-indicator">Creating backup...</div>

    <div class="backup-settings">
        <h2>Scheduled Backups</h2>
        <form hx-put="/api/backup/config">
            <div class="form-group">
                <label>
                    <input type="checkbox" name="auto_backup_enabled"
                           {% if config.auto_backup_enabled %}checked{% endif %}>
                    Enable automatic backups
                </label>
            </div>

            <div class="form-group">
                <label>Backup Schedule</label>
                <select name="backup_schedule">
                    <option value="daily">Daily (2:00 AM)</option>
                    <option value="weekly">Weekly (Sunday 2:00 AM)</option>
                    <option value="monthly">Monthly (1st day, 2:00 AM)</option>
                </select>
            </div>

            <div class="form-group">
                <label>Keep Last</label>
                <input type="number" name="keep_backups" value="{{ config.keep_backups }}" min="1">
                <small>Number of backups to retain</small>
            </div>

            <button type="submit" class="btn btn-primary">Save Settings</button>
        </form>
    </div>

    <div class="backup-list">
        <h2>Available Backups</h2>
        <table>
            <thead>
                <tr>
                    <th>Date</th>
                    <th>Size</th>
                    <th>Contents</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
                {% for backup in backups %}
                <tr>
                    <td>{{ backup.created_at | format_datetime }}</td>
                    <td>{{ backup.size | format_bytes }}</td>
                    <td>
                        <button hx-get="/api/backup/{{ backup.id }}/info"
                                hx-target="#backup-info-modal"
                                class="btn btn-sm btn-link">
                            View Details
                        </button>
                    </td>
                    <td>
                        <a href="/api/backup/{{ backup.id }}/download"
                           class="btn btn-sm">
                            ⬇ Download
                        </a>
                        <button hx-post="/api/backup/restore"
                                hx-vals='{"backup_id": "{{ backup.id }}"}'
                                hx-confirm="Restore from this backup? This will overwrite current configuration."
                                class="btn btn-sm btn-warning">
                            🔄 Restore
                        </button>
                        <button hx-delete="/api/backup/{{ backup.id }}"
                                hx-confirm="Delete this backup?"
                                hx-target="closest tr"
                                hx-swap="outerHTML swap:1s"
                                class="btn btn-sm btn-danger">
                            🗑 Delete
                        </button>
                    </td>
                </tr>
                {% endfor %}
            </tbody>
        </table>
    </div>
</div>

<div id="backup-info-modal" class="modal"></div>
```

---

## 7. Audit Log Viewer (`/admin/audit`)

**Features**:
- List all audit log entries
- Filter by action type
- Filter by user
- Filter by date range
- Search log entries
- Export logs (CSV, JSON)
- Color-coded by severity

**Template**:
```html
<div class="audit-page">
    <h1>Audit Log</h1>

    <div class="audit-filters">
        <input type="text"
               name="search"
               placeholder="Search logs..."
               hx-get="/api/auth/audit"
               hx-trigger="keyup changed delay:500ms"
               hx-target="#audit-table">

        <select name="action"
                hx-get="/api/auth/audit"
                hx-trigger="change"
                hx-target="#audit-table">
            <option value="">All Actions</option>
            <option value="login">Login</option>
            <option value="logout">Logout</option>
            <option value="create_user">Create User</option>
            <option value="delete_user">Delete User</option>
            <option value="password_change">Password Change</option>
            <option value="access_denied">Access Denied</option>
        </select>

        <select name="user"
                hx-get="/api/auth/audit"
                hx-trigger="change"
                hx-target="#audit-table">
            <option value="">All Users</option>
            {% for user in users %}
            <option value="{{ user.id }}">{{ user.username }}</option>
            {% endfor %}
        </select>

        <button hx-get="/api/auth/audit/export?format=csv" class="btn">
            📥 Export CSV
        </button>
    </div>

    <div id="audit-table">
        <table>
            <thead>
                <tr>
                    <th>Timestamp</th>
                    <th>User</th>
                    <th>Action</th>
                    <th>Resource</th>
                    <th>IP Address</th>
                    <th>Status</th>
                    <th>Details</th>
                </tr>
            </thead>
            <tbody>
                {% for entry in audit_log %}
                <tr class="audit-{{ entry.severity }}">
                    <td>{{ entry.timestamp | format_datetime }}</td>
                    <td>{{ entry.user }}</td>
                    <td><code>{{ entry.action }}</code></td>
                    <td>{{ entry.resource }}</td>
                    <td>{{ entry.ip_address }}</td>
                    <td>
                        {% if entry.success %}
                        <span class="status status-ok">Success</span>
                        {% else %}
                        <span class="status status-error">Failed</span>
                        {% endif %}
                    </td>
                    <td>{{ entry.details }}</td>
                </tr>
                {% endfor %}
            </tbody>
        </table>
    </div>
</div>
```

---

## 8. Database Management UI (`/admin/database`)

**Features**:
- Database connection status
- Database size and stats
- Run migrations
- Database backup/restore
- Query interface (admin only, dangerous)
- Table browser

**Template**:
```html
<div class="database-page">
    <h1>Database Management</h1>

    <div class="db-status">
        <div class="stat-card">
            <h3>Connection Status</h3>
            <span class="status status-ok">Connected</span>
        </div>

        <div class="stat-card">
            <h3>Database Type</h3>
            <span>{{ db_type }}</span>
        </div>

        <div class="stat-card">
            <h3>Database Size</h3>
            <span>{{ db_size | format_bytes }}</span>
        </div>

        <div class="stat-card">
            <h3>Tables</h3>
            <span>{{ table_count }}</span>
        </div>
    </div>

    <div class="db-actions">
        <button hx-post="/api/database/migrate"
                hx-target="#migration-status"
                class="btn btn-primary">
            Run Migrations
        </button>

        <button hx-post="/api/database/backup"
                class="btn btn-secondary">
            Backup Database
        </button>

        <button hx-get="/admin/database/query"
                hx-target="#query-interface"
                class="btn btn-warning">
            SQL Query Interface
        </button>
    </div>

    <div id="migration-status"></div>
    <div id="query-interface"></div>

    <div class="table-browser">
        <h2>Tables</h2>
        <ul>
            {% for table in tables %}
            <li>
                <button hx-get="/api/database/tables/{{ table.name }}"
                        hx-target="#table-data">
                    {{ table.name }} ({{ table.row_count }} rows)
                </button>
            </li>
            {% endfor %}
        </ul>
        <div id="table-data"></div>
    </div>
</div>
```

---

## 9. Security Settings UI (`/settings/security`)

**Features**:
- Session timeout configuration
- Password policy settings
- Account lockout settings
- JWT secret rotation
- Security event log
- Allowed IP whitelist

**Template**:
```html
<div class="security-settings">
    <h1>Security Settings</h1>

    <form hx-put="/api/settings/security">
        <section>
            <h2>Session Settings</h2>
            <div class="form-group">
                <label>Session Timeout (minutes)</label>
                <input type="number" name="session_timeout" value="{{ config.session_timeout }}">
            </div>

            <div class="form-group">
                <label>
                    <input type="checkbox" name="remember_me_enabled"
                           {% if config.remember_me_enabled %}checked{% endif %}>
                    Allow "Remember Me"
                </label>
            </div>
        </section>

        <section>
            <h2>Password Policy</h2>
            <div class="form-group">
                <label>Minimum Password Length</label>
                <input type="number" name="min_password_length" value="{{ config.min_password_length }}">
            </div>

            <div class="form-group">
                <label>
                    <input type="checkbox" name="require_uppercase"
                           {% if config.require_uppercase %}checked{% endif %}>
                    Require uppercase letters
                </label>
            </div>

            <div class="form-group">
                <label>
                    <input type="checkbox" name="require_numbers"
                           {% if config.require_numbers %}checked{% endif %}>
                    Require numbers
                </label>
            </div>

            <div class="form-group">
                <label>
                    <input type="checkbox" name="require_special_chars"
                           {% if config.require_special_chars %}checked{% endif %}>
                    Require special characters
                </label>
            </div>
        </section>

        <section>
            <h2>Account Lockout</h2>
            <div class="form-group">
                <label>Max Failed Login Attempts</label>
                <input type="number" name="max_login_attempts" value="{{ config.max_login_attempts }}">
            </div>

            <div class="form-group">
                <label>Lockout Duration (minutes)</label>
                <input type="number" name="lockout_duration" value="{{ config.lockout_duration }}">
            </div>
        </section>

        <section>
            <h2>API Security</h2>
            <div class="form-group">
                <label>API Rate Limit (requests per minute)</label>
                <input type="number" name="api_rate_limit" value="{{ config.api_rate_limit }}">
            </div>
        </section>

        <button type="submit" class="btn btn-primary">Save Settings</button>
    </form>
</div>
```

---

## 10. Enhanced Dashboard (`/` or `/dashboard`)

**Update existing dashboard to include**:
- Authentication status
- Recent audit events
- System health summary
- Scheduled task status
- Backup status
- Quick actions panel

---

## Implementation Plan

### Week 1-2: Authentication UI
- [ ] Login page with error handling
- [ ] User management page (list, create, edit, delete)
- [ ] API key management page
- [ ] User profile page

### Week 3: Health & Monitoring UI
- [ ] System health dashboard with gauges
- [ ] Real-time metrics updates
- [ ] Service status indicators
- [ ] Performance graphs (Chart.js or similar)

### Week 4: Email & Tasks UI
- [ ] Email configuration page
- [ ] Email testing and history
- [ ] Task scheduler page (list, create, edit)
- [ ] Task execution history viewer

### Week 5: Diagnostics & Backup UI
- [ ] Network diagnostics tools
- [ ] Backup creation and listing
- [ ] Restore interface with confirmation
- [ ] Backup scheduling configuration

### Week 6: Admin UI
- [ ] Audit log viewer with filters
- [ ] Database management page
- [ ] Security settings page
- [ ] Enhanced dashboard

### Week 7: Polish & Testing
- [ ] CSS styling and theming
- [ ] Responsive design
- [ ] Error handling and validation
- [ ] Integration testing
- [ ] Accessibility improvements

---

## CSS Framework

Continue using custom CSS with enhancements:

**New CSS files needed**:
- `static/css/auth.css` - Login page and auth forms
- `static/css/dashboard.css` - Dashboard widgets and gauges
- `static/css/diagnostics.css` - Diagnostic tools styling
- `static/css/admin.css` - Admin pages (audit log, database)

**CSS Components**:
- `.gauge` - Circular gauge for metrics
- `.metric-card` - Dashboard metric cards
- `.status-ok`, `.status-error`, `.status-warning` - Status indicators
- `.modal` - Modal dialogs
- `.alert` - Alert messages
- `.toggle` - Toggle switches
- `.badge` - Role/permission badges

---

## JavaScript Libraries

Minimal JavaScript additions:

**Chart.js** (for graphs):
```html
<script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
```

**Custom JS** (`static/js/app.js`):
- Modal open/close functions
- Copy to clipboard
- Password visibility toggle
- Chart initialization
- Real-time updates helpers

---

## Success Criteria

- [ ] All backend APIs have corresponding UI
- [ ] Consistent design across all pages
- [ ] Responsive layout (desktop, tablet, mobile)
- [ ] Proper error handling and user feedback
- [ ] Loading indicators for async operations
- [ ] Accessibility compliance (ARIA labels, keyboard navigation)
- [ ] No JavaScript required for core functionality (progressive enhancement)
- [ ] All forms have validation
- [ ] Confirmation dialogs for destructive actions
- [ ] User documentation for each feature

---

## Dependencies

**Rust Dependencies** (already in use):
- Askama templates
- Axum for routing
- Tower for middleware

**Frontend Dependencies**:
- HTMX (already included)
- Chart.js (new - for graphs)
- Custom CSS (expand existing)

**No heavy frameworks** - keep it lightweight and fast!

---

## Security Considerations

1. **CSRF Protection**: Add CSRF tokens to all forms
2. **XSS Prevention**: Askama auto-escapes, but validate user input
3. **Authentication**: Protect admin routes with auth middleware
4. **Authorization**: Check user roles/permissions before showing actions
5. **Rate Limiting**: Protect login and API key creation endpoints
6. **Input Validation**: Validate all form inputs on backend
7. **Secure Cookies**: HttpOnly, Secure, SameSite flags on session cookies

---

## Next Steps

1. Create issue tracking for each UI component
2. Design mockups for key pages
3. Set up CSS framework structure
4. Implement authentication UI first (highest priority)
5. Iterate based on user feedback

---

## Notes

- Keep templates simple and maintainable
- Use HTMX for interactivity (avoid heavy JS frameworks)
- Follow existing design patterns from current web UI
- Progressive enhancement (works without JavaScript)
- Mobile-first responsive design
- Accessibility is a requirement, not optional
