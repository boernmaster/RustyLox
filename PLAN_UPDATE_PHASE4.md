# Phase 4: Web UI Implementation Plan

## Phase 4a: Authentication & Credentials (Priority)

### Miniserver Credentials Management
- Add username/password fields to Miniserver configuration UI
- Secure credential storage (consider encryption for passwords)
- Update `MiniserverConfig` to include credentials
- Test connection with credentials

### MQTT Broker Credentials
- Add username/password to MQTT configuration
- Update `MqttConfig` to include authentication
- Test broker connection with credentials
- Support for anonymous vs authenticated modes

## Phase 4b: Web UI Core (Askama + HTMX)

### Technology Stack
- **Askama**: Server-rendered templates
- **HTMX**: Dynamic interactions without JavaScript framework
- **CSS Framework**: Tailwind CSS or Bootstrap (lightweight)
- **Real-time**: Server-Sent Events (SSE) for MQTT monitor

### Page Structure

#### 1. Dashboard (`/`)
- System status overview
- Miniserver connection status
- MQTT gateway status
- Recent plugin activity
- System resources (memory, disk)

#### 2. Miniserver Management (`/miniserver`)
- List all configured Miniservers
- Add/Edit/Delete Miniserver
- Test connection button
- Credentials management (username/password)
- CloudDNS configuration

#### 3. MQTT Monitor (`/mqtt/monitor`) **[Like Original]**
- **Live message stream** (real-time updates)
- Topic filter/search
- Message payload display
- Timestamp for each message
- Pause/Resume streaming
- Clear message history
- Export messages (CSV/JSON)
- WebSocket or SSE for real-time updates

#### 4. MQTT Configuration (`/mqtt/config`)
- Broker settings (host, port, credentials)
- UDP listener configuration
- Subscription management (add/edit/delete)
- Transformer configuration

#### 5. Plugin Management (`/plugins`)
- List installed plugins
- Upload/Install new plugin (ZIP)
- Uninstall plugin
- Plugin details view
- Plugin logs

#### 6. Settings (`/settings`)
- General configuration
- Language selection
- Time zone settings
- Update settings
- Backup/Restore

### UI Components

#### Navigation
```html
<nav>
  <a href="/">Dashboard</a>
  <a href="/miniserver">Miniserver</a>
  <a href="/mqtt/monitor">MQTT Monitor</a>
  <a href="/mqtt/config">MQTT Config</a>
  <a href="/plugins">Plugins</a>
  <a href="/settings">Settings</a>
</nav>
```

#### MQTT Monitor Component (Real-time)
```rust
// Server-Sent Events for live MQTT messages
async fn mqtt_monitor_stream() -> Sse<impl Stream<Item = Event>> {
    // Stream MQTT messages to browser in real-time
}
```

## Implementation Order

### Step 1: Web UI Crate Setup
- Create `crates/web-ui/` with Askama
- Set up templates directory
- Configure HTMX integration
- Add Tailwind CSS

### Step 2: Authentication (Phase 4a)
- Add credential fields to configuration structs
- Create credential management UI
- Test with real Miniserver
- Add MQTT authentication

### Step 3: Basic Pages
- Dashboard (read-only status)
- Navigation layout
- Settings page

### Step 4: MQTT Monitor (Priority)
- Real-time message stream (SSE or WebSocket)
- Message filtering
- Pause/Resume controls
- Export functionality

### Step 5: Management Pages
- Miniserver CRUD operations
- Plugin installation UI
- MQTT subscription management

### Step 6: Polish
- Responsive design
- Error handling
- Loading states
- Notifications/toasts

## MQTT Monitor Requirements (Original-like)

Based on the original LoxBerry MQTT monitor, implement:

### Display Features
- **Message Table**: Topic, Payload, QoS, Retain, Timestamp
- **Auto-scroll**: New messages appear at top or bottom
- **Color coding**: Different topics have different colors
- **Search/Filter**: Filter by topic pattern
- **Payload formatting**: JSON pretty-print, XML formatting

### Controls
- **Pause/Resume**: Stop auto-update but keep connection
- **Clear**: Clear message history
- **Max Messages**: Limit displayed messages (e.g., last 100)
- **Topic Filter**: Subscribe to specific topics for monitoring

### Real-time Updates
- WebSocket or Server-Sent Events
- Minimal latency (< 100ms)
- Efficient rendering (virtual scrolling for large lists)

## Technical Details

### Askama Template Example
```rust
#[derive(Template)]
#[template(path = "mqtt_monitor.html")]
struct MqttMonitorTemplate {
    messages: Vec<MqttMessage>,
    paused: bool,
    filter: String,
}
```

### HTMX Example
```html
<!-- Auto-updating status -->
<div hx-get="/api/mqtt/status"
     hx-trigger="every 2s"
     hx-swap="innerHTML">
  Loading...
</div>

<!-- Form submission without page reload -->
<form hx-post="/mqtt/subscribe"
      hx-target="#subscriptions">
  <input name="topic" placeholder="home/#">
  <button type="submit">Subscribe</button>
</form>
```

### SSE for MQTT Monitor
```rust
async fn mqtt_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.mqtt_gateway.subscribe_to_messages();

    Sse::new(ReceiverStream::new(rx).map(|msg| {
        Ok(Event::default()
            .data(serde_json::to_string(&msg).unwrap()))
    }))
}
```

## Dependencies to Add

```toml
# web-ui/Cargo.toml
askama = { version = "0.12", features = ["with-axum"] }
askama_axum = "0.4"
tower-http = { version = "0.5", features = ["fs"] } # Static files
```

## File Structure

```
crates/web-ui/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── handlers/
│   │   ├── dashboard.rs
│   │   ├── miniserver.rs
│   │   ├── mqtt.rs
│   │   ├── plugins.rs
│   │   └── settings.rs
│   └── templates/
│       ├── base.html          # Base layout
│       ├── dashboard.html
│       ├── miniserver/
│       │   ├── list.html
│       │   ├── edit.html
│       │   └── add.html
│       ├── mqtt/
│       │   ├── monitor.html   # MQTT Monitor (real-time)
│       │   ├── config.html
│       │   └── subscriptions.html
│       ├── plugins/
│       │   ├── list.html
│       │   ├── install.html
│       │   └── details.html
│       └── settings.html
└── static/
    ├── css/
    │   └── style.css
    ├── js/
    │   └── htmx.min.js
    └── img/
```

## Success Criteria

- [ ] Phase 4a: Credentials working for Miniserver & MQTT
- [ ] Dashboard shows real-time status
- [ ] MQTT Monitor displays live messages (like original)
- [ ] Miniserver management (add/edit/delete) functional
- [ ] Plugin installation via web UI works
- [ ] Responsive design (works on mobile)
- [ ] All forms use HTMX (no full page reloads)
- [ ] Error handling with user-friendly messages
