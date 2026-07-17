# Containerized Addons for RustyLox

**Status:** Draft, approved by user 2026-07-17
**Author:** boernmaster + Claude

## Motivation

RustyLox's existing plugin system implements the LoxBerry ZIP format — `plugin.cfg`
manifest, lifecycle hook scripts, a Perl/PHP/Bash daemon RustyLox spawns and
controls as a subprocess inside its own container/filesystem (`crates/plugin-manager`).
That model doesn't fit addons that are their own independent Docker containers —
the first real example is `kia-connect-bridge`
(https://github.com/boernmaster/kia-connect-bridge), which bridges Kia Connect
vehicle SOC/range into the same MQTT broker RustyLox relays to Loxone, and
already runs as a standalone `docker compose` service with its own config web UI.

This spec defines a second, separate addon model — for containerized addons —
that sits alongside the existing ZIP-plugin system without replacing or
touching it.

## Explicitly rejected alternatives

- **Full Docker lifecycle control from RustyLox** (RustyLox starts/stops/upgrades
  addon containers via the Docker API). Rejected: requires mounting
  `docker.sock` (or a restricted proxy) into RustyLox, giving it control over
  containers beyond just addons. The user wants to keep deploying/managing
  Docker themselves and only wants RustyLox to surface settings for containers
  that are already running.
- **docker.sock (direct or via docker-socket-proxy) for read-only discovery.**
  Rejected for the same reason — no Docker access of any kind from RustyLox in
  this design, not even read-only inspection.
- **Local hand-maintained catalog file.** Superseded — the catalog is a live
  query against GHCR instead of a file the user edits by hand.

## Architecture

Two independent concepts, both new:

### 1. Catalog — discovery of addon *types*

A live view of what containerized addons exist to deploy, sourced from GitHub
Container Registry under `ghcr.io/boernmaster/*` — not a hand-maintained list.
An image is treated as a RustyLox addon if its OCI labels include
`io.rustylox.addon=true`. RustyLox calls GitHub's Packages API
(`GET /users/boernmaster/packages?package_type=container`) using a configured
PAT, filters to labeled images, and reads each one's standard OCI annotations
(`org.opencontainers.image.title`, `.description`, `.source`) for display.

The catalog is pure discovery — it never deploys anything. Each entry shows a
copyable `docker run` / compose snippet pointing at the image; the user runs
it themselves.

### 2. Registry — live addon *instances*

A view of addon containers that are actually running and have announced
themselves. Protocol:

- On startup, and every 60s thereafter (heartbeat), the addon `POST`s to
  RustyLox's `/api/addons/register` with:
  ```json
  {"name": "kia-connect-bridge", "version": "1.0.0", "config_api_base_url": "http://10.0.0.32:8090"}
  ```
- RustyLox keeps registered instances in memory, persisted to a JSON file
  (same pattern as `plugindatabase.json`), and marks an instance offline if no
  heartbeat arrives within 3x the interval (~3 minutes).
- No authentication on the register endpoint — same LAN-trust model as the
  rest of RustyLox (plain HTTP, not internet-facing). Malformed payloads are
  rejected with 400 and logged, not stored.

### Addon config contract

So RustyLox can render settings for *any* registered addon without addon-specific
code, each addon exposes 3 endpoints (generalizing `kia-connect-bridge`'s
existing `webui.py` `FIELDS`/`read_env`/`write_env` logic into JSON instead of
server-rendered HTML):

- `GET /addon/schema` → field descriptors: `[{"key", "label", "type", "help", "secret": bool}, ...]`
- `GET /addon/config` → current values; secret fields return `""` with a
  `"secret_set": true` flag instead of the real value (never round-trip
  secrets, same rule already followed in `webui.py`)
- `POST /addon/config` → save; a blank value for a secret field means "keep
  the existing one" (same semantics `webui.py` already has)

The addon's own HTML form (if any) is unaffected — this is purely an
additional JSON API for RustyLox to consume.

## RustyLox-side changes

New crate `crates/addon-registry`, parallel to `plugin-manager` (not merged
into it — different lifecycle model, no filesystem/daemon management):

- **Catalog module** — GitHub Packages API client, OCI-label filtering,
  ~10 minute cache. If `GITHUB_PACKAGES_TOKEN` isn't set, the catalog tab
  reports "not configured"; nothing else in the addon system depends on it.
- **Registry module** — `POST /api/addons/register`, `GET /api/addons`
  (list with online/offline status), JSON-file persistence, staleness sweep.
- **Proxy module** — `GET/POST /api/addons/:name/schema`,
  `GET/POST /api/addons/:name/config` — forwards to the instance's
  `config_api_base_url`; returns a clean "addon offline" error (not a hang or
  crash) if the instance is unreachable.

New env vars (both optional — absence just disables the catalog tab):
- `GITHUB_PACKAGES_TOKEN` — GitHub PAT, `read:packages` scope
- `GITHUB_PACKAGES_USER` — defaults to `boernmaster`

**web-ui**: new "Addons" page (distinct from the existing "Plugins" page),
two tabs:
- **Catalog** — cards: title, description, source link, copyable deploy
  snippet.
- **Installed** — registered live instances, generic settings form rendered
  from schema + config, online/offline badge.

## kia-connect-bridge changes (pilot addon)

- **New `.github/workflows/ci.yml`**, modeled on RustyLox's own (tag-triggered,
  `docker/build-push-action`, built-in `GITHUB_TOKEN` for push — no PAT needed
  to *publish*, only RustyLox's catalog *read* needs the PAT). Pushes to
  `ghcr.io/boernmaster/kia-connect-bridge`.
- **Dockerfile**: add labels `io.rustylox.addon=true`,
  `org.opencontainers.image.title=kia-connect-bridge`,
  `org.opencontainers.image.description=Bridges Kia Connect vehicle SOC/range/charging state into MQTT for Loxone`,
  `org.opencontainers.image.source=https://github.com/boernmaster/kia-connect-bridge`.
- **`webui.py`**: add `GET /addon/schema`, `GET /addon/config`,
  `POST /addon/config`, thin JSON wrappers around the existing `FIELDS`,
  `read_env`, `write_env`. Existing HTML form at `/` is unchanged.
- **`run.py`**: new background thread posting the registration heartbeat to
  `RUSTYLOX_URL` (new env var). Empty/unset = heartbeat disabled, container
  still works exactly as it does today standalone.

## Data flow

```
docker compose up -d (addon, anywhere on the LAN)
        |
        v
  POST /api/addons/register  (heartbeat, every 60s)  ---->  RustyLox addon-registry
                                                                     |
User opens RustyLox "Addons -> Installed" tab  <--------------------+
        |
        v
  GET schema+config, POST config changes  -- proxied -->  addon's /addon/config

Independently:
  tag+push to ghcr.io/boernmaster/<addon>  ---->  RustyLox "Addons -> Catalog" tab
  (pure discovery, no relation to whether anything is currently running)
```

## Error handling

- GitHub Packages API unreachable/rate-limited: catalog tab shows the last
  cached result with a "stale" indicator; never blocks or crashes the rest of
  RustyLox.
- Addon heartbeat malformed: `400`, logged, not stored.
- Registered addon goes offline: marked offline in the Installed tab; config
  proxy calls return a clear "addon offline" error instead of hanging.
- Two instances registering the same `name`: last-write-wins (handles
  container restarts / IP changes on the LAN).

## Testing

- **kia-connect-bridge**: tests for the 3 new endpoints — schema shape,
  config round-trip, secret masking, blank-secret-keeps-existing-value.
- **RustyLox**: integration test analogous to
  `crates/plugin-manager/tests/weather4lox_install.rs` — spin up a fake addon
  HTTP server, exercise register → list → proxy GET/POST, verify staleness
  marks it offline, verify a mocked GitHub Packages API response is correctly
  filtered by the `io.rustylox.addon` label for the catalog.

## Security notes

- No Docker access anywhere in this design (no `docker.sock`, direct or
  proxied) — deploying/updating/removing addon containers stays entirely the
  user's manual responsibility.
- Register endpoint is unauthenticated, consistent with RustyLox's existing
  LAN-only trust model (plain HTTP, not internet-facing). Not appropriate if
  RustyLox is ever exposed beyond a trusted LAN.
- `GITHUB_PACKAGES_TOKEN` only needs `read:packages` — it cannot push, delete,
  or modify anything in GHCR.

## Out of scope for this spec

- Addon uninstall/removal from the Installed list (stale entries just show
  offline indefinitely for now — cleanup can be a follow-up).
- Multi-host addon discovery beyond simple heartbeat (no mDNS/service mesh).
- Any UI/flow for actually *deploying* a catalog entry — the catalog only
  shows a copyable snippet.
