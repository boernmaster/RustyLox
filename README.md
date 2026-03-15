# LoxBerry Rust - Phase 1

Modern Rust rewrite of LoxBerry with Docker containerization.

## Phase 1 Status

✅ **Completed Components:**
- Core types and error handling (`loxberry-core`)
- Configuration management (`loxberry-config`)
- Miniserver HTTP/UDP client (`miniserver-client`)
- REST API (`web-api`)
- Main daemon (`loxberry-daemon`)
- Docker setup

## Architecture

```
loxberry-rust/
├── crates/
│   ├── loxberry-core/       - Common types and errors
│   ├── loxberry-config/     - JSON config management
│   ├── miniserver-client/   - HTTP/UDP Miniserver communication
│   ├── web-api/             - REST API with Axum
│   └── loxberry-daemon/     - Main orchestrator binary
├── Dockerfile               - Multi-stage build
└── docker-compose.yml       - Docker Compose configuration
```

## Features

### Miniserver Client
- HTTP/HTTPS communication with Basic Auth
- UDP messaging (max 220 bytes)
- Delta-sending optimization (only send changed values)
- Miniserver reboot detection
- SSL certificate verification disabled (for self-signed certs)

### REST API
- `GET /health` - Health check
- `GET /api/config/general` - Get configuration
- `PUT /api/config/general` - Update configuration
- `GET /api/miniserver` - List all Miniservers
- `GET /api/miniserver/:id` - Get Miniserver details
- `POST /api/miniserver/:id/send` - Send HTTP command
- `POST /api/miniserver/:id/get` - Get values
- `GET /api/miniserver/:id/status` - Check connection status
- `GET /api/system/status` - System status

## Building

### Prerequisites
- Rust 1.80+ (if building locally)
- Docker and Docker Compose (for containerized deployment)

### Local Development
```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run daemon locally
LBHOMEDIR=/path/to/config cargo run --bin loxberry-daemon
```

### Docker Deployment
```bash
# Build and start containers
docker-compose up -d

# View logs
docker-compose logs -f loxberry

# Stop containers
docker-compose down
```

## Configuration

Configuration is stored in JSON format at `/opt/loxberry/config/system/general.json`.

Example minimal configuration:
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
      "Admin_raw": "admin",
      "Pass": "password",
      "Pass_raw": "password",
      "Credentials": "admin:password",
      "Credentials_raw": "admin:password",
      "Transport": "http",
      "Useclouddns": "0"
    }
  },
  "Mqtt": {
    "Brokerhost": "localhost",
    "Brokerport": "1883",
    "Udpinport": "11884",
    "Uselocalbroker": "1",
    "Websocketport": "9001",
    "Finderdisabled": false
  }
}
```

## Testing

### HTTP Client Test
```bash
# Send command to Miniserver
curl -X POST http://localhost:8080/api/miniserver/1/send \
  -H "Content-Type: application/json" \
  -d '{
    "params": [
      {"parameter": "V1", "value": "100"}
    ]
  }'

# Get values from Miniserver
curl -X POST http://localhost:8080/api/miniserver/1/get \
  -H "Content-Type: application/json" \
  -d '{
    "params": ["Temperature", "Humidity"]
  }'

# Check Miniserver status
curl http://localhost:8080/api/miniserver/1/status
```

### System Status
```bash
# Health check
curl http://localhost:8080/health

# System status
curl http://localhost:8080/api/system/status
```

## Volume Mounts

When using Docker, the following directories are mounted as volumes:
- `./volumes/config` → `/opt/loxberry/config`
- `./volumes/data` → `/opt/loxberry/data`
- `./volumes/log` → `/opt/loxberry/log`

Create a configuration file before starting:
```bash
mkdir -p volumes/config/system
# Copy or create general.json in volumes/config/system/
```

## Next Steps (Phase 2)

- [ ] Plugin manager (install/uninstall)
- [ ] Plugin database (JSON)
- [ ] Lifecycle hook execution
- [ ] Plugin API endpoints

## Next Steps (Phase 3)

- [ ] MQTT Gateway implementation
- [ ] UDP input listener
- [ ] Message transformers
- [ ] Bidirectional relay

## License

Same as original LoxBerry project.
