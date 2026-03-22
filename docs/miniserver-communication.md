# Miniserver Communication Guide

This document explains how to set up bidirectional communication between a
Loxone Miniserver and RustyLox.

## Overview

RustyLox supports three methods for the Miniserver to **send data to** RustyLox
and two methods for RustyLox to **send data to** the Miniserver.

| Direction               | Protocol     | Port  | Loxone Config Element         |
|-------------------------|-------------|-------|-------------------------------|
| Miniserver -> RustyLox  | HTTP        | 8080  | Virtual Output (HTTP)         |
| Miniserver -> RustyLox  | UDP         | 8090  | Virtual Output (UDP)          |
| Miniserver -> RustyLox  | UDP         | 11884 | Virtual Output (UDP)          |
| RustyLox -> Miniserver  | HTTP        | 80    | Virtual Input (HTTP)          |
| RustyLox -> Miniserver  | UDP         | *     | Virtual Input (UDP)           |

---

## 1. Miniserver Sending Data TO RustyLox

### Option A: Virtual HTTP Output (Recommended)

This is the most reliable method. The Miniserver makes an HTTP GET request to
RustyLox whenever data changes.

#### Setup in Loxone Config

1. **Create a Virtual Output**
   - Go to: Periphery > Virtual Outputs > Add Virtual Output
   - Address: `http://<RustyLox-IP>:8080`
   - Leave "Close connection after sending" **enabled**

2. **Add Virtual Output Commands**
   - Click the Virtual Output, then add a Virtual Output Command
   - **Command on**: `/dev/sps/io/<Name>/<v>`
   - Replace `<Name>` with your chosen name (e.g., `LivingRoom_Temperature`)
   - `<v>` is the Loxone placeholder that gets replaced with the actual value

3. **Connect to a function block**
   - Wire the analog or digital output of your function block to the Virtual
     Output Command input

#### How it works

When the Miniserver triggers the command, it sends:
```
GET http://<RustyLox-IP>:8080/dev/sps/io/LivingRoom_Temperature/23.5
```

RustyLox:
- Returns a Loxone-compatible XML response: `<LL value="23.5" Code="200"/>`
- Publishes the value to MQTT as `LivingRoom/Temperature = 23.5`
  (underscores are converted to MQTT topic separators)
- Shows the event in the Miniserver Monitor UI

#### Example Loxone Config commands

| Use case          | Command on                           |
|-------------------|--------------------------------------|
| Analog value      | `/dev/sps/io/Sensor_Temperature/<v>` |
| Digital on/off    | `/dev/sps/io/Light_Kitchen/1`        |
| Digital off       | `/dev/sps/io/Light_Kitchen/0`        |
| Text value        | `/dev/sps/io/Status_Text/<v>`        |

### Option B: Virtual UDP Output

Lighter weight than HTTP but without delivery confirmation.

#### Setup in Loxone Config

1. **Create a Virtual Output**
   - Address: `/dev/udp/<RustyLox-IP>/8090`
   - Enable "Close connection after sending"

2. **Add Virtual Output Commands**
   - **Command on**: `SensorData: Temperature=<v>`

   You can send multiple values in one packet:
   - **Command on**: `Weather: Temp=<v1> Humidity=<v2>`

#### How it works

The Miniserver sends a UDP packet to RustyLox port 8090:
```
SensorData: Temperature=23.5
```

RustyLox parses the `prefix: key=value` format and:
- Publishes to MQTT as `SensorData/Temperature = 23.5`
- Shows the event in the Miniserver Monitor

#### Alternative: Send to MQTT Gateway UDP port (11884)

You can also send to port 11884 (the MQTT Gateway UDP input). This port
accepts three formats:

| Format                                    | Example                                    |
|-------------------------------------------|--------------------------------------------|
| JSON                                      | `{"topic":"home/temp","value":"23.5"}`     |
| Simple key=value                          | `home/temp=23.5`                           |
| Miniserver prefix format                  | `Weather: Temp=23.5 Humidity=65`           |

To use port 11884, set the Virtual Output address to:
```
/dev/udp/<RustyLox-IP>/11884
```

---

## 2. RustyLox Sending Data TO Miniserver

### Option A: HTTP Virtual Input (Automatic via MQTT Gateway)

When RustyLox receives an MQTT message that matches a subscription, it
automatically sends it to the Miniserver via HTTP.

#### Setup in Loxone Config

1. **Create a Virtual Input**
   - Name it matching the MQTT topic with underscores instead of slashes
   - Example: MQTT topic `home/temperature` -> Virtual Input name `home_temperature`

2. **Configure in RustyLox**
   - Ensure the MQTT Gateway is enabled (UDP port > 0 in MQTT settings)
   - Add subscriptions in `config/system/mqtt_subscriptions.cfg`

#### How it works

```
MQTT message arrives: home/temperature = 23.5
  -> RustyLox calls: http://<Miniserver>/dev/sps/io/home_temperature/23.5
  -> Miniserver updates the Virtual Input
```

### Option B: UDP Virtual Input

If a Miniserver has a `udpport` configured (in the Miniserver settings page),
RustyLox also sends values via UDP in the original LoxBerry MQTT Gateway
format:

```
MQTT: home/temperature=23.5
```

#### Setup in Loxone Config

1. **Create a Virtual UDP Input**
   - Set the UDP port matching the one configured in RustyLox Miniserver
     settings (e.g., 7044)

2. **Add Command Recognition**
   - In the Virtual UDP Input properties, add command recognition entries
   - Example: `MQTT: home/temperature=\v` to extract the temperature value

---

## 3. Ports Summary

| Port  | Protocol | Direction          | Description                                |
|-------|----------|--------------------|--------------------------------------------|
| 8080  | TCP/HTTP | Miniserver->RustyLox | Virtual HTTP Output endpoint (`/dev/sps/io/`) |
| 8090  | UDP      | Miniserver->RustyLox | Dedicated Miniserver Virtual UDP Output receiver |
| 11884 | UDP      | Both               | MQTT Gateway UDP interface (also accepts Miniserver format) |
| 6066  | TCP/HTTP | Miniserver->RustyLox | Loxone Cloud Emulator (weather.loxone.com) |

### Environment Variables

| Variable            | Default | Description                              |
|---------------------|---------|------------------------------------------|
| `MS_UDP_RECV_PORT`  | `8090`  | Port for Miniserver Virtual UDP Output   |
| `BIND_ADDR`         | `0.0.0.0:8080` | HTTP server bind address          |

---

## 4. Monitoring

All inbound and outbound Miniserver communication is visible in the
**Miniserver Monitor** UI:

```
http://<RustyLox-IP>:8080/miniserver/monitor
```

Events are color-coded:
- **Blue**: Sent (RustyLox -> Miniserver)
- **Green**: Received (Miniserver -> RustyLox)
- **Red**: Errors

---

## 5. Troubleshooting

### Messages from Miniserver not arriving

1. **Check the Monitor UI** - Open `/miniserver/monitor` to see if any events
   appear
2. **Check network connectivity** - Ensure the Miniserver can reach RustyLox:
   - HTTP: `curl http://<RustyLox-IP>:8080/dev/sps/io/test/1`
   - UDP: Use Loxone Config UDP Monitor to verify packets are sent
3. **Check ports** - Ensure ports 8080, 8090, and 11884 are not blocked by
   firewalls. In Docker, verify the ports are exposed in `docker-compose.yml`
4. **Check logs** - Set `RUST_LOG=debug` and look for "Virtual HTTP Input
   received" or "UDP from" messages

### Messages not reaching Miniserver

1. **Check Virtual Input names** - The name must match the MQTT topic with
   slashes replaced by underscores
2. **Check credentials** - Verify admin/password in the Miniserver settings
3. **Test connection** - Use the Miniserver list page to test connectivity
4. **Check the topic filter** - The MQTT `Topicfilter` setting may be blocking
   certain topics

### Common mistakes

- **Wrong address format**: Use `http://<IP>:8080` (not `https://`) for the
  Virtual Output address in Loxone Config
- **Missing port in UDP address**: Must be `/dev/udp/<IP>/8090` (not just
  `/dev/udp/<IP>`)
- **Trailing slash**: Do not add a trailing slash after the port in UDP
  addresses
- **Virtual Input naming**: Use underscores, not slashes:
  `home_temperature` (not `home/temperature`)

---

## 6. Architecture Diagram

```
                    Loxone Miniserver
                   /        |        \
                  /         |         \
    Virtual HTTP   Virtual UDP    Virtual UDP
     Output          Output         Input
        |              |              ^
        v              v              |
  +-----------+  +-----------+  +-----------+
  | Port 8080 |  | Port 8090 |  | HTTP/UDP  |
  | /dev/sps/ |  | UDP recv  |  | send      |
  +-----------+  +-----------+  +-----------+
        |              |              ^
        v              v              |
  +------------------------------------+
  |         RustyLox MQTT Gateway       |
  |  - Transform pipeline               |
  |  - MQTT publish/subscribe           |
  |  - Miniserver relay                 |
  +------------------------------------+
        |              ^
        v              |
  +------------------------------------+
  |          MQTT Broker               |
  |        (Mosquitto)                 |
  +------------------------------------+
```

## References

- [Loxone Virtual Inputs & Outputs](https://www.loxone.com/enen/kb/virtual-inputs-outputs/)
- [Loxone UDP Communication](https://www.loxone.com/enen/kb/communication-with-udp/)
- [LoxBerry MQTT Gateway](https://wiki.loxberry.de/plugins/lox2mqtt/start)
- [LoxBerry msudp_send](https://wiki.loxberry.de/entwickler/php_develop_plugins_with_php/php_loxberry_sdk_documentation/php_module_loxberry_iophp/msudp_send)
