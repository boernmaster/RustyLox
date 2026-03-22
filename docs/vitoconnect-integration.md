# Vitoconnect Plugin Integration

This document describes how the Viessmann Vitoconnect plugin integrates with
RustyLox and the Loxone Miniserver. It covers data flow in both directions:
reading sensor data from the Viessmann API and sending commands from the
Miniserver to control the heating system.

## Overview

The Vitoconnect plugin acts as a bridge between the Viessmann cloud API and your
Loxone Miniserver. It supports two independent communication paths:

```
Viessmann Cloud API
        ^        |
        |        | (1) Periodic polling (cron)
   SET  |        v
commands|   vitoconnect.php
        |        |
        |        +---> MQTT Broker ---> Miniserver (via MQTT Gateway)
        |        +---> HTTP direct push to Miniserver Virtual Inputs
        |
Loxone Miniserver
   (Virtual HTTP Output calls vitoconnect.php?action=setvalue)
```

## Reading Data (Viessmann -> Miniserver)

### How It Works

1. **Cron job** triggers `vitoconnect.php` at a configured interval (1-60 min)
2. The script authenticates with the Viessmann API (OAuth2) and fetches all
   feature data for the installation
3. Data is parsed into key-value pairs (e.g. `heating.dhw.temperature.main.value = 55`)
4. Values are relayed to the Miniserver via one or both of:
   - **MQTT**: Published to `vitoconnect/<key>` topics on the broker
   - **HTTP**: Pushed directly to Miniserver Virtual Inputs via `mshttp_send()`

### MQTT Topic Structure

The plugin publishes to the MQTT broker with the configured base topic (default:
`vitoconnect`). Dots in Viessmann feature names are converted to slashes:

```
vitoconnect/heating/dhw/temperature/main/value = 55
vitoconnect/heating/boiler/temperature/value = 48.3
vitoconnect/heating/circuits/0/operating/modes/active/value = dhwAndHeating
vitoconnect/heating/circuits/0/operating/modes/active/value/enum = 2
vitoconnect/general/aggregatedstatus = WorksProperly
vitoconnect/general/aggregatedstatus_ok = 1
```

The MQTT Gateway (RustyLox) subscribes to `vitoconnect/#` and relays these
values to the Miniserver as Virtual Inputs. Topic slashes are converted to
underscores for the Miniserver parameter name:

```
MQTT topic: vitoconnect/heating/dhw/temperature/main/value
  -> Miniserver VI name: vitoconnect_heating_dhw_temperature_main_value
```

### Plugin Configuration

The plugin config file is stored at:
```
/opt/loxberry/config/plugins/vitoconnect/config.json  (inside Docker)
volumes/config/plugins/vitoconnect/config.json         (host mount)
```

Example config:
```json
{
  "user": "your-viessmann-email@example.com",
  "pass": "your-password",
  "apikey": "your-api-key-from-developer-portal",
  "apiversion": "v2",
  "Cron": {
    "enabled": true,
    "interval": "05min"
  },
  "MQTT": {
    "enabled": true,
    "topic": "vitoconnect",
    "host": "",
    "user": "",
    "pass": ""
  },
  "Loxone": {
    "enabled": false,
    "msnr": 1,
    "cachedisabled": false
  }
}
```

If `MQTT.host` is left empty, the plugin automatically reads broker connection
details from the MQTT Gateway plugin (recommended setup).

## Sending Commands (Miniserver -> Viessmann)

### How It Works

The Miniserver sends commands to the Viessmann API by calling the plugin's PHP
script directly via HTTP. This does **not** go through MQTT.

```
Miniserver (Virtual HTTP Output)
    |
    | HTTP GET with Basic Auth
    v
RustyLox Web Server (:8080)
    |
    | Route: /admin/plugins/Vitoconnect/vitoconnect.php
    | Executed via php-cgi
    v
vitoconnect.php
    |
    | POST to Viessmann Cloud API
    | e.g. .../heating.dhw.oneTimeCharge/commands/activate
    v
Viessmann Cloud API -> Heating system
```

### URL Format

```
http://<user>:<password>@<RUSTYLOX_IP>:8080/admin/plugins/Vitoconnect/vitoconnect.php?action=setvalue&option=<PARAMETER>&value=<VALUE>
```

- `<user>:<password>` - RustyLox HTTP Basic Auth credentials
- `<RUSTYLOX_IP>` - IP address of the RustyLox host
- `8080` - RustyLox web server port (adjust if using a reverse proxy)
- `action=setvalue` - tells the plugin to send a command
- `option` - the Viessmann feature path (see table below)
- `value` - the value to set

### Example: One-Time Hot Water Charge

**Activate:**
```
http://admin:pass@10.0.0.7:8080/admin/plugins/Vitoconnect/vitoconnect.php?action=setvalue&option=heating.dhw.oneTimeCharge&value=start
```

**Deactivate:**
```
http://admin:pass@10.0.0.7:8080/admin/plugins/Vitoconnect/vitoconnect.php?action=setvalue&option=heating.dhw.oneTimeCharge&value=stop
```

### Loxone Config Setup

1. Create a **Virtual HTTP Output** (or use a **Virtual Output Command**)
2. Set the URL to the format above
3. Connect it to a function block (button, schedule, etc.) to trigger the call

### Supported Set-Value Parameters

#### Hot Water (DHW)

| option | value | Description |
|---|---|---|
| `heating.dhw.oneTimeCharge` | `start` / `stop` | Activate/deactivate one-time hot water charge |
| `heating.dhw.temperature.main` | number (e.g. `55`) | Set main DHW target temperature |
| `heating.dhw.temperature` | number | Set DHW temperature |
| `heating.dhw.temperature.temp2` | number | Set secondary DHW temperature |
| `heating.dhw.temperature.hysteresis` | number | Set DHW hysteresis value |
| `heating.dhw.operating.modes.active` | `off` / `eco` / `comfort` | Set DHW operating mode |
| `heating.dhw.hygiene` | `enable` | Enable DHW hygiene (only in comfort mode) |
| `heating.dhw.schedule` | JSON schedule | Set DHW schedule |
| `heating.dhw.pumps.circulation.schedule` | JSON schedule | Set circulation pump schedule |

#### Heating Circuits (replace `0` with `1` or `2` for other circuits)

| option | value | Description |
|---|---|---|
| `heating.circuits.0.operating.modes.active` | `standby` / `dhwAndHeating` / `dhw` / `forcedNormal` / `forcedReduced` | Set operating mode (string) |
| `heating.circuits.0.operating.modes.active.enum` | `1`-`5` | Set operating mode (numeric, for Loxone) |
| `heating.circuits.0.operating.programs.normal` | number | Normal program target temperature |
| `heating.circuits.0.operating.programs.reduced` | number | Reduced program target temperature |
| `heating.circuits.0.operating.programs.comfort` | number | Comfort program target temperature |
| `heating.circuits.0.operating.programs.reducedHeating` | number | Reduced heating temperature |
| `heating.circuits.0.operating.programs.normalHeating` | number | Normal heating temperature |
| `heating.circuits.0.operating.programs.comfortHeating` | number | Comfort heating temperature |
| `heating.circuits.0.operating.programs.reducedCooling` | number | Reduced cooling temperature |
| `heating.circuits.0.operating.programs.normalCooling` | number | Normal cooling temperature |
| `heating.circuits.0.operating.programs.comfortCooling` | number | Comfort cooling temperature |
| `heating.circuits.0.operating.programs.forcedLastFromSchedule` | `activate` / `deactivate` | Force last schedule program |
| `heating.circuits.0.heating.curve` | `shift\|slope` | Set heating curve (pipe-separated values) |
| `heating.circuits.0.heating.schedule` | JSON schedule | Set heating schedule |
| `heating.circuits.0.temperature.levels.setMin` | number | Set minimum temperature level |
| `heating.circuits.0.temperature.levels.setMax` | number | Set maximum temperature level |
| `heating.circuits.0.temperature.levels.setLevels` | `min\|max` | Set both temperature levels (pipe-separated) |
| `heating.circuits.0.name` | string | Set circuit name |

#### Operating Mode Enum Values

When using the `.enum` variants (designed for Loxone numeric inputs):

| Enum Value | Operating Mode |
|---|---|
| 1 | standby |
| 2 | dhwAndHeating |
| 3 | dhw |
| 4 | forcedNormal |
| 5 | forcedReduced |

Note: Value `0` is silently ignored (Loxone sometimes sends 0 as a "no choice" default).

#### Ventilation

| option | value | Description |
|---|---|---|
| `ventilation.operating.modes.active.enum` | `1`-`3` | Set ventilation mode (1=standby, 2=standard, 3=ventilation) |
| `ventilation.quickmodes.comfort` | `activate` / `deactivate` | Toggle comfort quick mode |
| `ventilation.quickmodes.eco` | `activate` / `deactivate` | Toggle eco quick mode |
| `ventilation.schedule` | JSON schedule | Set ventilation schedule |
| `ventilation.schedule.resetSchedule` | (any) | Reset ventilation schedule to default |

#### Holiday Programs

| option | value | Description |
|---|---|---|
| `heating.operating.programs.holiday.schedule` | `start\|end` | Schedule a holiday program (pipe-separated ISO dates) |
| `heating.operating.programs.holiday.unschedule` | (any) | Cancel holiday program |
| `heating.operating.programs.holidayAtHome.schedule` | `start\|end` | Schedule holiday-at-home (pipe-separated ISO dates) |
| `heating.operating.programs.holidayAtHome.unschedule` | (any) | Cancel holiday-at-home program |

## Other Actions

### Fetch Summary Data (Manual Trigger)

Triggers a full data fetch from the Viessmann API and publishes all values:

```
http://admin:pass@10.0.0.7:8080/admin/plugins/Vitoconnect/vitoconnect.php?action=summary
```

This is the same action the cron job runs automatically.

### Force Re-Login

Discards the cached OAuth2 token and performs a fresh login:

```
http://admin:pass@10.0.0.7:8080/admin/plugins/Vitoconnect/vitoconnect.php?action=relogin
```

## Prerequisites

- A Viessmann Developer Portal account with an API key
  (https://developer.viessmann.com/)
- A Viessmann Vitoconnect device connected to your heating system
- RustyLox running with the Vitoconnect plugin installed
- MQTT broker (mosquitto) if using MQTT data transfer

## Troubleshooting

### Commands Not Reaching Viessmann

1. Check the plugin log at `volumes/log/plugins/vitoconnect/` for errors
2. Verify the OAuth2 token is valid (try `action=relogin`)
3. Ensure the API key has write permissions on the Viessmann Developer Portal
4. Check that the `option` parameter matches exactly (case-sensitive)

### Data Not Appearing on Miniserver

1. Verify cron is enabled in the plugin settings
2. Check MQTT broker connectivity: `mosquitto_sub -t "vitoconnect/#" -v`
3. Ensure the MQTT subscription is configured in
   `volumes/config/system/mqtt_subscriptions.cfg`:
   ```ini
   [Vitoconnect]
   TOPIC=vitoconnect/#
   NAME=Viessmann Heating
   ENABLED=1
   ```
4. Check that Virtual Inputs on the Miniserver match the parameter names
   (underscored format: `vitoconnect_heating_dhw_temperature_main_value`)

### Token Expiration

The plugin caches the OAuth2 token in `/run/shm/` (ramdisk). If the token
expires, the plugin automatically re-authenticates on the next run. If
authentication fails repeatedly, use `action=relogin` to force a fresh login.
