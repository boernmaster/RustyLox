# Configuration

## Deploy from Pre-Built Image

Pull and run without cloning the repository:

```bash
# Download the example compose file
curl -fsSL https://raw.githubusercontent.com/boernmaster/RustyLox/main/docker-compose.example.yml \
  -o docker-compose.yml

# Start RustyLox + Mosquitto
docker compose up -d
```

With an external MQTT broker, edit the environment section before starting:

```yaml
environment:
  - MQTT_BROKER=192.168.1.10
  - MQTT_PORT=1883
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LBHOMEDIR` | `/opt/loxberry` | Base directory for config, data, and log |
| `RUST_LOG` | `info` | Log level (`error`, `warn`, `info`, `debug`, `trace`) |
| `BIND_ADDR` | `0.0.0.0:80` | HTTP bind address |
| `MQTT_BROKER` | `mosquitto` | MQTT broker hostname |
| `MQTT_PORT` | `1883` | MQTT broker port |

---

## Volume Mounts

| Host path | Container path | Purpose |
|-----------|---------------|---------|
| `./volumes/config` | `/opt/loxberry/config` | JSON and INI config files |
| `./volumes/data` | `/opt/loxberry/data` | Plugin data, backup archives |
| `./volumes/log` | `/opt/loxberry/log` | Log files |
| `./volumes/webfrontend` | `/opt/loxberry/webfrontend` | Plugin web frontends |
| `./volumes/templates` | `/opt/loxberry/templates/plugins` | Plugin templates |
| `./volumes/bin` | `/opt/loxberry/bin/plugins` | Plugin executables |

---

## Ports

| Port | Protocol | Service |
|------|----------|---------|
| `80` | TCP | Web UI and REST API |
| `6066` | TCP | Loxone Cloud Emulator (weather service) |
| `53` | UDP | DNS redirect (dnsmasq) |
| `53` | TCP | DNS TCP |
| `8090` | UDP | Miniserver Virtual UDP Output receiver |
| `11884` | UDP | MQTT UDP input gateway |
| `1883` | TCP | Mosquitto MQTT broker |
| `9001` | TCP | Mosquitto WebSocket |

---

## general.json

The main configuration file lives at `$LBHOMEDIR/config/system/general.json`. On first start with no existing file, RustyLox creates defaults automatically.

### Full Example

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
      "Useclouddns": "0",
      "Transport": "http",
      "Porthttps": "443"
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

### Field Reference

#### Base

| Key | Default | Description |
|-----|---------|-------------|
| `Lang` | `en` | UI language (`en`, `de`, `fr`, `es`) |
| `Systemloglevel` | `6` | Log verbosity (1 = error … 7 = trace) |
| `Clouddnsuri` | `dns.loxonecloud.com` | CloudDNS hostname for Miniserver lookup |
| `Version` | `4.0.0.0` | Config schema version (do not change) |

#### Miniserver

Multiple Miniservers are supported. The key is a numeric string (`"1"`, `"2"`, …).

| Key | Description |
|-----|-------------|
| `Name` | Display name |
| `Ipaddress` | IP address or hostname |
| `Port` | HTTP port (default `80`) |
| `Porthttps` | HTTPS port (default `443`) |
| `Transport` | `http` or `https` |
| `Admin` | Username for Basic Auth |
| `Pass` | Password for Basic Auth |
| `Useclouddns` | `"1"` to resolve IP via CloudDNS |

#### Mqtt

| Key | Default | Description |
|-----|---------|-------------|
| `Brokerhost` | `mosquitto` | MQTT broker hostname |
| `Brokerport` | `1883` | MQTT broker port |
| `Brokeruser` | `""` | Broker username (leave empty if no auth) |
| `Brokerpass` | `""` | Broker password |
| `Udpinport` | `11884` | UDP input port (`0` to disable) |
| `Uselocalbroker` | `"1"` | `"1"` = use built-in Mosquitto |
| `Websocketport` | `9001` | Mosquitto WebSocket port |

#### Timeserver

| Key | Default | Description |
|-----|---------|-------------|
| `Timezone` | `Europe/Vienna` | IANA timezone string |

---

## MQTT Subscriptions

`$LBHOMEDIR/config/system/mqtt_subscriptions.cfg` — INI format, one section per subscription.

```ini
[HomeTemperature]
TOPIC=home/+/temperature
NAME=Temperature Sensors
FILTER=_healthcheck_|_info_
ENABLED=1
```

| Key | Description |
|-----|-------------|
| `TOPIC` | MQTT topic pattern (supports `+` and `#` wildcards) |
| `NAME` | Human-readable label shown in the UI |
| `FILTER` | Pipe-separated regex patterns to suppress matching messages |
| `ENABLED` | `1` to subscribe, `0` to disable without deleting |

Reload at runtime without restarting:

```bash
curl -X POST http://localhost/api/mqtt/subscriptions/reload
```

---

## MQTT Transformers

`$LBHOMEDIR/config/system/mqtt_transformers.cfg` — INI format, one section per rule.

```ini
[JsonExpand]
TYPE=json_expand
ENABLED=1

[BoolConvert]
TYPE=bool_convert
ENABLED=1
```

Reload at runtime:

```bash
curl -X POST http://localhost/api/mqtt/transformers/reload
```

---

## Backup Schedule

Configured via the UI at **Backup → Schedule** or via the API:

```bash
curl -X PUT http://localhost/api/backup/schedule \
  -H "Content-Type: application/json" \
  -d '{"active": true, "interval_hours": 24, "keep_backups": 7}'
```

---

## Troubleshooting

**Service won't start**

```bash
docker compose logs rustylox
```

Common causes: missing or corrupt `general.json` (delete it to regenerate defaults); port 80 already in use (change `BIND_ADDR`).

**MQTT messages not appearing**

1. Check broker: `docker compose ps mosquitto`
2. Verify `Brokerhost` and `Brokerport` in Settings → MQTT Config
3. Check `volumes/config/system/mqtt_subscriptions.cfg`

**Plugin hook fails**

1. Check plugin logs in `volumes/log/plugins/<name>/`
2. Verify Perl/PHP is available: `docker compose exec rustylox perl -v`

**Uninstalling**

```bash
docker compose down -v   # remove containers and named volumes
rm -rf volumes/          # remove all persistent data (irreversible)
```
