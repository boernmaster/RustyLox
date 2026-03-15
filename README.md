# LoxBerry Rust

Modern Rust rewrite of LoxBerry with Docker containerization.

## About

This project is a complete rewrite of [LoxBerry](https://github.com/mschlenstedt/Loxberry) in Rust. LoxBerry is an open-source toolbox for Raspberry Pi that extends the Loxone Smart Home System with additional features like MQTT integration, weather services, and a plugin ecosystem.

**Original Repository:** https://github.com/mschlenstedt/Loxberry
**License:** Same as original LoxBerry project

## Project Status

✅ **Phase 1 - Foundation** (Completed)
- Core types and error handling
- Configuration management (JSON)
- Miniserver HTTP/UDP client
- REST API foundation

✅ **Phase 2 - Plugin System** (Completed)
- Plugin manager (install/uninstall/upgrade)
- Plugin database (JSON)
- Lifecycle hooks (preroot, preinstall, postinstall, postroot, uninstall)
- Plugin API endpoints

✅ **Phase 3 - MQTT Gateway** (Completed)
- MQTT broker integration (rumqttc)
- UDP input listener (port 11884)
- Message transformers (built-in + external scripts)
- Bidirectional relay (MQTT ↔ Miniserver)
- Hot-reload for subscriptions and transformers

✅ **Phase 4 - Web UI** (Completed)
- Server-rendered templates (Askama)
- HTMX for dynamic interactions
- Real-time MQTT monitor (Server-Sent Events)
- Miniserver management interface
- Plugin management interface
- System settings page

✅ **Phase 4a - Authentication** (Completed)
- Miniserver credentials (username/password)
- MQTT broker authentication
- Credential storage ready

## Architecture

```
loxberry-rust/
├── crates/
│   ├── loxberry-core/       - Common types and errors
│   ├── loxberry-config/     - JSON config management
│   ├── miniserver-client/   - HTTP/UDP Miniserver communication
│   ├── mqtt-gateway/        - MQTT Gateway with transformers
│   ├── plugin-manager/      - Plugin lifecycle management
│   ├── web-api/             - REST API with Axum
│   ├── web-ui/              - Server-rendered web interface (Askama + HTMX)
│   └── loxberry-daemon/     - Main orchestrator binary
├── static/                  - CSS and JavaScript assets
├── volumes/                 - Docker volume mounts
│   ├── config/              - Configuration files
│   ├── data/                - Data storage
│   └── log/                 - Log files
├── Dockerfile               - Multi-stage build
└── docker-compose.yml       - Stack with Mosquitto broker
```

## Features

### Miniserver Client
- HTTP/HTTPS communication with Basic Auth
- UDP messaging (max 220 bytes)
- Delta-sending optimization (only send changed values)
- Miniserver reboot detection
- CloudDNS dynamic IP resolution
- SSL certificate verification disabled (for self-signed certs)

### MQTT Gateway
- Connect to Mosquitto broker (or external MQTT broker)
- UDP input listener on port 11884
- Topic subscription management
- Message transformer pipeline:
  - JSON expansion
  - Boolean conversion (true/false/on/off → 1/0)
  - Custom external scripts (Perl/PHP/Bash)
- Bidirectional relay (MQTT ↔ Miniserver)
- Hot-reload for configuration changes

### Plugin System
- Install/uninstall/upgrade plugins from ZIP archives
- Plugin database (JSON at `data/system/plugindatabase.json`)
- MD5-based plugin identity (author + name + folder)
- Lifecycle hooks with script execution
- Directory isolation per plugin
- Environment variable injection for SDK compatibility

### Web UI
- **Dashboard** - System overview and quick links
- **Miniserver Management** - Add, edit, delete, test connections
- **MQTT Monitor** - Real-time message viewer with SSE streaming
- **MQTT Configuration** - Broker settings and authentication
- **Plugin Management** - Install, list, uninstall plugins
- **Settings** - System configuration
- Server-rendered templates (Askama)
- HTMX for progressive enhancement
- Responsive CSS design

### REST API

#### Configuration
- `GET /api/config/general` - Get general.json
- `PUT /api/config/general` - Update configuration

#### Miniserver
- `GET /api/miniserver` - List all Miniservers
- `GET /api/miniserver/:id` - Get Miniserver details
- `POST /api/miniserver/:id/send` - Send HTTP command
- `POST /api/miniserver/:id/get` - Get values
- `POST /api/miniserver/:id/udp` - Send UDP command
- `GET /api/miniserver/:id/status` - Check connection status

#### Plugins
- `GET /api/plugins` - List all installed plugins
- `GET /api/plugins/:md5` - Get plugin details
- `POST /api/plugins/install` - Install plugin from ZIP
- `DELETE /api/plugins/:md5` - Uninstall plugin
- `POST /api/plugins/:md5/upgrade` - Upgrade plugin

#### MQTT Gateway
- `GET /api/mqtt/status` - Gateway status (connected, subscriptions, transformers)
- `POST /api/mqtt/subscriptions/reload` - Hot-reload subscriptions
- `POST /api/mqtt/transformers/reload` - Hot-reload transformers

#### System
- `GET /health` - Health check
- `GET /api/system/status` - System status

## Quick Start

### Prerequisites
- Docker and Docker Compose
- (Optional) Rust 1.80+ for local development

### 1. Clone and Setup
```bash
git clone <this-repo-url>
cd loxberry-rust

# Create volume directories
mkdir -p volumes/config/system volumes/data/system volumes/log/system

# Copy default configuration (or create your own)
cp volumes/config/system/general.json.example volumes/config/system/general.json
# Edit volumes/config/system/general.json with your Miniserver details
```

### 2. Build and Start
```bash
# Build and start all containers
docker compose up -d

# View logs
docker compose logs -f loxberry

# Check status
docker compose ps
```

### 3. Access Web UI
Open your browser to **http://localhost:8080/**

- Dashboard: http://localhost:8080/
- MQTT Monitor: http://localhost:8080/mqtt/monitor
- Miniserver Management: http://localhost:8080/miniserver
- Settings: http://localhost:8080/settings

## Configuration

Configuration is stored in JSON format at `volumes/config/system/general.json`.

Example configuration:
```json
{
  "Base": {
    "Clouddnsuri": "dns.loxonecloud.com",
    "Lang": "en",
    "Sendstatistic": 1,
    "Startsetup": "1",
    "Systemloglevel": "6",
    "Version": "4.0.0.0"
  },
  "Miniserver": {
    "1": {
      "Name": "My Miniserver",
      "Ipaddress": "192.168.1.100",
      "Port": "80",
      "Admin": "admin",
      "Pass": "password",
      "Useclouddns": "0"
    }
  },
  "Mqtt": {
    "Brokerhost": "mosquitto",
    "Brokerport": "1883",
    "Brokeruser": "",
    "Brokerpass": "",
    "Udpinport": "11884",
    "Uselocalbroker": "1",
    "Websocketport": "9001"
  },
  "Timeserver": {
    "Timezone": "Europe/Vienna"
  }
}
```

### MQTT Subscriptions

MQTT subscriptions are configured in `volumes/config/system/mqtt_subscriptions.cfg`:

```ini
[home/#]
[sensors/+/temperature]
```

## Testing

### Web UI
```bash
# Open browser to:
http://localhost:8080/

# Test MQTT monitor real-time streaming:
http://localhost:8080/mqtt/monitor
```

### REST API
```bash
# Health check
curl http://localhost:8080/health

# System status
curl http://localhost:8080/api/system/status

# MQTT Gateway status
curl http://localhost:8080/api/mqtt/status

# Send command to Miniserver
curl -X POST http://localhost:8080/api/miniserver/1/send \
  -H "Content-Type: application/json" \
  -d '{"params": [{"V1": "100"}]}'

# List plugins
curl http://localhost:8080/api/plugins
```

### MQTT Testing
```bash
# Publish message to MQTT broker
docker exec mosquitto mosquitto_pub -t "home/test" -m "Hello from MQTT"

# Subscribe to all messages
docker exec mosquitto mosquitto_sub -t "#" -v

# Send MQTT message via UDP gateway
echo '{"topic":"home/sensor1","value":"25.5"}' | nc -u localhost 11884
```

## Docker Services

The `docker-compose.yml` defines two services:

1. **loxberry** - Main LoxBerry Rust application
   - Port 8080: Web UI and REST API
   - Port 11884/udp: MQTT UDP input

2. **mosquitto** - Eclipse Mosquitto MQTT broker
   - Port 1883: MQTT broker
   - Port 9001: WebSocket

## Volume Mounts

When using Docker, the following directories are mounted as volumes:

- `./volumes/config` → `/opt/loxberry/config` (Configuration files)
- `./volumes/data` → `/opt/loxberry/data` (Plugin data, database)
- `./volumes/log` → `/opt/loxberry/log` (Log files)
- `./volumes/plugins` → `/opt/loxberry/plugins` (Plugin files)

## Development

### Local Build
```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run daemon locally
LBHOMEDIR=/tmp/loxberry cargo run --bin loxberry-daemon
```

### Docker Build
```bash
# Rebuild Docker image
docker compose build loxberry

# Restart services
docker compose down && docker compose up -d
```

### Project Structure
Each crate is independent with its own Cargo.toml:

- **loxberry-core**: Common types, errors, results
- **loxberry-config**: JSON config parsing and management
- **miniserver-client**: HTTP/UDP client for Loxone Miniserver
- **mqtt-gateway**: MQTT broker client, UDP listener, transformers
- **plugin-manager**: ZIP extraction, database, lifecycle hooks
- **web-api**: REST API routes and handlers (Axum)
- **web-ui**: Server-rendered templates and UI handlers (Askama)
- **loxberry-daemon**: Main binary that orchestrates all services

## Differences from Original LoxBerry

### Technology Stack
- **Language**: Rust (vs. Perl/PHP/Bash)
- **Runtime**: Single compiled binary (vs. multiple interpreted scripts)
- **Async**: Tokio async runtime (vs. synchronous scripts)
- **Container**: Docker (vs. bare metal Raspberry Pi)

### Architecture
- **Modular**: Separate crates for each component
- **Type-Safe**: Rust's type system prevents many runtime errors
- **Performance**: Compiled code with zero-cost abstractions
- **Modern Web**: Askama templates + HTMX (vs. CGI scripts)

### Compatibility
- **Configuration**: Compatible JSON format with original LoxBerry
- **Plugins**: SDK compatibility layer in development (Phase 5)
- **MQTT**: Full compatibility with existing MQTT clients

## Roadmap

### Phase 5 (Future)
- [ ] Full SDK compatibility layer (Perl/PHP/Bash)
- [ ] Plugin migration tools
- [ ] Legacy CGI plugin support
- [ ] Plugin marketplace integration

### Phase 6 (Future)
- [ ] System update mechanism
- [ ] Backup/restore functionality
- [ ] Email notifications
- [ ] Logging framework with rotation

## Contributing

This is a rewrite based on the original LoxBerry project. Contributions are welcome!

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## Support

- **Original LoxBerry**: https://www.loxwiki.eu/display/LOXBERRY/LoxBerry
- **Forum**: https://www.loxforum.com/forum/german/software-konfiguration-und-programmierung/loxberry
- **GitHub Issues**: <this-repo-issues-url>

## Acknowledgments

Based on the original **LoxBerry** project by Christian Fenzl and the LoxBerry community. Special thanks to all contributors of the original project for creating such a comprehensive smart home platform.

## License

Same license as the original LoxBerry project.
