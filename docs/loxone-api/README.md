# Loxone API Documentation

This folder contains markdown-converted Loxone API documentation for use in RustyLox development.

## Documents

| File | Source | Description |
|------|--------|-------------|
| [miniserver-communication.md](miniserver-communication.md) | Loxone PDF datasheet v16.0 (2025-06-03) | WebSocket & HTTP communication protocol, authentication (tokens, JWT), encryption, message formats, event tables, structure file |
| [pms-access-api.md](pms-access-api.md) | `pms-access-api.zip` (OpenAPI 3.0.3 spec) | PMS & Access API v1.0.1 — user configuration, NFC tag assignment, door opening, room moods, room status |
| [web-services.md](web-services.md) | https://www.loxone.com/enen/kb/web-services/ (2025-02-04) | HTTP web service commands — status queries, control commands, PLC ops, config, system monitoring, device commands |

## Quick Reference

### Miniserver Base URL

```
http://{miniserver-ip}/jdev/...
https://{miniserver-ip}/jdev/...  (Gen 2 / Compact with TLS)
```

### PMS & Access API Base URL

```
https://miniserver-ip/jdev/sps
```

### Authentication (Miniserver)

- Token-based auth (since v9.0, updated v10.2) — recommended
- JWT tokens (since v10.2) — preferred
- HTTP Basic Auth — debugging only

### Authentication (PMS & Access API)

- `hospitality_auth` — HTTP Basic
- `bearer_auth` — JWT Bearer token

## Key Endpoints (PMS & Access API)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/jdev/sps/getgrouplist` | List all user groups |
| POST | `/jdev/sps/configureuser` | Create/update user + assign NFC tag |
| GET | `/jdev/sps/startnfclearning/{uuid}` | Start NFC learning mode |
| GET | `/jdev/sps/io/{uuid}/pulse/{outputNr}` | Open door (unsecured) |
| GET | `/jdev/sps/ios/{uuid}/pulse/{outputNr}/{hash}` | Open door (secured) |
| GET | `/jdev/sps/io/{ircUuid}/setComfortTemperature/{temp}` | Set room temperature |
| GET | `/jdev/sps/io/{roomstatusUuid}/AQ/{statusId}` | Set room status |
| GET | `/jdev/sps/io/{roomstatusUuid}/GetOutput` | Get current room status |
