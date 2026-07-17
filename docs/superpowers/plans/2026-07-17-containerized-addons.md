# Containerized Addons Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let RustyLox discover containerized addons (via a live GHCR catalog) and manage the settings of already-running addon instances (via self-registration + a generic config API), without RustyLox ever touching Docker itself.

**Architecture:** A new `crates/addon-registry` crate holds three independent pieces — an in-memory + JSON-persisted registry of self-registered live instances, a reqwest-based proxy that forwards schema/config calls to a registered instance's own config API, and a GitHub-Packages+GHCR client that builds the discovery catalog from images labeled `io.rustylox.addon=true`. `kia-connect-bridge` is the pilot addon: it gains a JSON config API (`/addon/schema`, `/addon/config`) alongside its existing HTML form, and a heartbeat thread that registers it with RustyLox.

**Tech Stack:** Rust (axum 0.7, reqwest 0.12, tokio, serde/serde_json, askama 0.12) for RustyLox; Python 3.12 stdlib (`http.server`) + pytest for kia-connect-bridge.

Full design context: `docs/superpowers/specs/2026-07-17-containerized-addons-design.md`.

## Global Constraints

- No Docker access from RustyLox anywhere in this feature (no `docker.sock`, direct or proxied) — deployment stays manual.
- `POST /api/addons/register` is unauthenticated, matching RustyLox's existing LAN-trust model (plain HTTP, not internet-facing).
- Heartbeat interval: 60s. An instance is marked offline after 3 missed heartbeats (~180s of silence).
- Catalog cache TTL: 10 minutes.
- `GITHUB_PACKAGES_TOKEN` only ever needs `read:packages` scope.
- Secrets never round-trip into a response — a field marked `secret` returns `""` with `secret_set: true`/`false` instead of its real value; submitting a blank value for a secret field means "keep the existing one".
- Addon config contract (implemented by every containerized addon, not just Kia): `GET /addon/schema`, `GET /addon/config`, `POST /addon/config`.

---

## Track A: kia-connect-bridge (pilot addon)

Repo: `/home/boern/projects/kia-connect-bridge`

### Task 1: Addon config JSON API

**Files:**
- Create: `addon_api.py`
- Create: `tests/test_addon_api.py`
- Create: `tests/__init__.py` (empty)
- Modify: `webui.py`
- Modify: `pyproject.toml`

**Interfaces:**
- Produces: `addon_api.build_schema() -> list[dict]`, `addon_api.build_config() -> dict[str, dict]`, `addon_api.save_config(submitted: dict) -> None` — consumed by `webui.py`'s `Handler`, and later by RustyLox's proxy (Task 7) as the JSON shape it forwards.

- [ ] **Step 1: Set up a local venv with uv and add pytest**

```bash
cd /home/boern/projects/kia-connect-bridge
uv venv
```

Add a dev dependency group to `pyproject.toml`:

```toml
[project]
name = "kia-connect-bridge"
version = "0.1.0"
requires-python = ">=3.12"
dependencies = [
    "hyundai-kia-connect-api>=1.60.0",
    "paho-mqtt>=2.1.0",
]

[dependency-groups]
dev = [
    "pytest>=8.0.0",
]
```

```bash
uv sync --group dev
```

- [ ] **Step 2: Write the failing tests**

Create `tests/__init__.py` (empty file) and `tests/test_addon_api.py`:

```python
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import addon_api
import webui


def test_build_schema_marks_secrets():
    schema = addon_api.build_schema()
    by_key = {f["key"]: f for f in schema}
    assert by_key["MQTT_PASSWORD"]["secret"] is True
    assert by_key["MQTT_HOST"]["secret"] is False
    assert by_key["MQTT_HOST"]["label"] == "MQTT Host"
    assert len(schema) == len(webui.FIELDS)


def test_build_config_masks_existing_secret(tmp_path, monkeypatch):
    env_path = tmp_path / ".env"
    env_path.write_text("KIA_PASSWORD=supersecret\nMQTT_HOST=10.0.0.99\n")
    monkeypatch.setattr(webui, "ENV_PATH", str(env_path))

    config = addon_api.build_config()

    assert config["KIA_PASSWORD"]["value"] == ""
    assert config["KIA_PASSWORD"]["secret_set"] is True
    assert config["MQTT_HOST"]["value"] == "10.0.0.99"
    assert config["MQTT_HOST"]["secret_set"] is False


def test_build_config_defaults_when_unset(tmp_path, monkeypatch):
    env_path = tmp_path / ".env"
    env_path.write_text("")
    monkeypatch.setattr(webui, "ENV_PATH", str(env_path))

    config = addon_api.build_config()

    assert config["MQTT_HOST"]["value"] == webui.DEFAULTS["MQTT_HOST"]
    assert config["MQTT_HOST"]["secret_set"] is False


def test_save_config_blank_secret_keeps_existing(tmp_path, monkeypatch):
    env_path = tmp_path / ".env"
    env_path.write_text("KIA_PASSWORD=supersecret\n")
    monkeypatch.setattr(webui, "ENV_PATH", str(env_path))

    addon_api.save_config({"KIA_PASSWORD": "", "MQTT_HOST": "10.0.0.50"})

    saved = webui.read_env(str(env_path))
    assert saved["KIA_PASSWORD"] == "supersecret"
    assert saved["MQTT_HOST"] == "10.0.0.50"


def test_save_config_updates_secret_when_provided(tmp_path, monkeypatch):
    env_path = tmp_path / ".env"
    env_path.write_text("KIA_PASSWORD=oldsecret\n")
    monkeypatch.setattr(webui, "ENV_PATH", str(env_path))

    addon_api.save_config({"KIA_PASSWORD": "newsecret"})

    saved = webui.read_env(str(env_path))
    assert saved["KIA_PASSWORD"] == "newsecret"
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd /home/boern/projects/kia-connect-bridge
uv run pytest tests/test_addon_api.py -v
```

Expected: `ModuleNotFoundError: No module named 'addon_api'`

- [ ] **Step 4: Write `addon_api.py`**

```python
"""JSON API for RustyLox's addon config contract.

GET /addon/schema, GET /addon/config, POST /addon/config - thin wrappers
around webui.py's existing FIELDS/DEFAULTS/SECRET_KEYS/read_env/write_env.
The addon's own HTML form at "/" is unaffected by any of this.
"""

import webui


def build_schema() -> list[dict]:
    return [
        {
            "key": key,
            "label": label,
            "type": ftype,
            "help": help_text,
            "secret": key in webui.SECRET_KEYS,
        }
        for key, label, ftype, help_text in webui.FIELDS
    ]


def build_config() -> dict:
    current = webui.read_env(webui.ENV_PATH)
    result = {}
    for key, _, _, _ in webui.FIELDS:
        if key in webui.SECRET_KEYS:
            result[key] = {"value": "", "secret_set": bool(current.get(key))}
        else:
            result[key] = {
                "value": current.get(key) or webui.DEFAULTS.get(key, ""),
                "secret_set": False,
            }
    return result


def save_config(submitted: dict) -> None:
    current = webui.read_env(webui.ENV_PATH)
    for key, _, _, _ in webui.FIELDS:
        value = submitted.get(key, "")
        if key in webui.SECRET_KEYS and not value:
            continue
        current[key] = value
    webui.write_env(webui.ENV_PATH, current)
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd /home/boern/projects/kia-connect-bridge
uv run pytest tests/test_addon_api.py -v
```

Expected: 5 passed

- [ ] **Step 6: Wire the endpoints into `webui.py`'s `Handler`**

In `webui.py`, add `import json` and `import addon_api` near the top (after the existing imports), add a `_respond_json` helper to `Handler`, and route the new paths in `do_GET`/`do_POST`:

```python
import html
import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from string import Template
from urllib.parse import parse_qs

import addon_api
```

```python
class Handler(BaseHTTPRequestHandler):
    def _respond(self, body: str):
        encoded = body.encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def _respond_json(self, data):
        encoded = json.dumps(data).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def do_GET(self):
        if self.path == "/addon/schema":
            self._respond_json(addon_api.build_schema())
        elif self.path == "/addon/config":
            self._respond_json(addon_api.build_config())
        else:
            self._respond(render_form())

    def do_POST(self):
        if self.path == "/addon/config":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length).decode()
            addon_api.save_config(json.loads(body))
            self._respond_json({"saved": True})
            return

        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length).decode()
        submitted = {k: v[0] for k, v in parse_qs(body).items()}

        current = read_env(ENV_PATH)
        for key, _, _, _ in FIELDS:
            value = submitted.get(key, "")
            if key in SECRET_KEYS and not value:
                continue  # blank means "keep the existing secret"
            current[key] = value
        write_env(ENV_PATH, current)

        self._respond(render_form(message='<div class="msg">Saved.</div>'))
```

- [ ] **Step 7: Run the full test suite**

```bash
cd /home/boern/projects/kia-connect-bridge
uv run pytest -v
```

Expected: all tests pass

- [ ] **Step 8: Commit**

```bash
cd /home/boern/projects/kia-connect-bridge
git add addon_api.py webui.py pyproject.toml tests/
git commit -m "feat: add JSON config API for RustyLox addon contract"
```

---

### Task 2: Registration heartbeat

**Files:**
- Create: `heartbeat.py`
- Create: `tests/test_heartbeat.py`
- Modify: `run.py`
- Modify: `.env.example`

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `heartbeat.py`'s `build_payload() -> dict`, `heartbeat_loop(rustylox_url: str, interval_seconds: int, poster) -> None` (a background-thread-safe loop; `poster` is an injected callable so tests never hit the network), `start_if_configured() -> threading.Thread | None` — consumed by `run.py`.

- [ ] **Step 1: Write the failing tests**

Create `tests/test_heartbeat.py`:

```python
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import heartbeat


def test_build_payload_shape(monkeypatch):
    monkeypatch.setenv("WEBUI_PORT", "8090")
    payload = heartbeat.build_payload()

    assert payload["name"] == "kia-connect-bridge"
    assert payload["version"]
    assert payload["config_api_base_url"].endswith(":8090")


def test_heartbeat_loop_posts_until_stopped():
    calls = []

    def fake_poster(url, payload):
        calls.append((url, payload))
        if len(calls) >= 3:
            raise heartbeat.StopHeartbeat

    heartbeat.heartbeat_loop(
        "http://rustylox.example:8080",
        interval_seconds=0,
        poster=fake_poster,
    )

    assert len(calls) == 3
    assert calls[0][0] == "http://rustylox.example:8080/api/addons/register"
    assert calls[0][1]["name"] == "kia-connect-bridge"


def test_start_if_configured_returns_none_when_unset(monkeypatch):
    monkeypatch.delenv("RUSTYLOX_URL", raising=False)
    assert heartbeat.start_if_configured() is None


def test_start_if_configured_starts_thread_when_set(monkeypatch):
    monkeypatch.setenv("RUSTYLOX_URL", "http://rustylox.example:8080")
    thread = heartbeat.start_if_configured()
    assert thread is not None
    assert thread.daemon is True
    thread.join(timeout=0.1)
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /home/boern/projects/kia-connect-bridge
uv run pytest tests/test_heartbeat.py -v
```

Expected: `ModuleNotFoundError: No module named 'heartbeat'`

- [ ] **Step 3: Write `heartbeat.py`**

```python
"""Self-registration heartbeat: tells RustyLox this addon exists and where
to reach its config API, so RustyLox never needs Docker access to find it.

Disabled entirely (start_if_configured returns None, no thread starts) when
RUSTYLOX_URL isn't set - the container still works fully standalone.
"""

import logging
import os
import threading
import time
import urllib.error
import urllib.request
import json

log = logging.getLogger("heartbeat")

VERSION = "1.0.0"
HEARTBEAT_INTERVAL_SECONDS = 60


class StopHeartbeat(Exception):
    """Raised by a poster (real or fake) to end the loop - used by tests."""


def build_payload() -> dict:
    port = os.environ.get("WEBUI_PORT", "8090")
    host = os.environ.get("ADDON_ADVERTISE_HOST") or _local_ip()
    return {
        "name": "kia-connect-bridge",
        "version": VERSION,
        "config_api_base_url": f"http://{host}:{port}",
    }


def _local_ip() -> str:
    import socket

    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(("10.255.255.255", 1))
        return s.getsockname()[0]
    except OSError:
        return "127.0.0.1"
    finally:
        s.close()


def _real_poster(url: str, payload: dict) -> None:
    data = json.dumps(payload).encode()
    request = urllib.request.Request(
        url, data=data, headers={"Content-Type": "application/json"}, method="POST"
    )
    try:
        urllib.request.urlopen(request, timeout=5)
    except urllib.error.URLError as e:
        log.warning("Heartbeat POST to %s failed: %s", url, e)


def heartbeat_loop(rustylox_url: str, interval_seconds: int = HEARTBEAT_INTERVAL_SECONDS, poster=_real_poster) -> None:
    url = f"{rustylox_url.rstrip('/')}/api/addons/register"
    while True:
        payload = build_payload()
        try:
            poster(url, payload)
        except StopHeartbeat:
            return
        time.sleep(interval_seconds)


def start_if_configured() -> threading.Thread | None:
    rustylox_url = os.environ.get("RUSTYLOX_URL", "").strip()
    if not rustylox_url:
        log.info("RUSTYLOX_URL not set - addon registration heartbeat disabled")
        return None
    thread = threading.Thread(target=heartbeat_loop, args=(rustylox_url,), daemon=True)
    thread.start()
    return thread
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /home/boern/projects/kia-connect-bridge
uv run pytest tests/test_heartbeat.py -v
```

Expected: 4 passed

- [ ] **Step 5: Wire into `run.py`**

Modify `run.py`:

```python
import logging
import os
import threading

import heartbeat
import kia_soc_bridge
import webui

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
log = logging.getLogger("run")


def main() -> None:
    if os.environ.get("KIA_USERNAME") and os.environ.get("KIA_PASSWORD"):
        threading.Thread(target=kia_soc_bridge.main, daemon=True).start()
    else:
        log.warning(
            "KIA_USERNAME/KIA_PASSWORD not set - fill them in via the web UI on :%s, "
            "then restart the container",
            webui.WEBUI_PORT,
        )

    heartbeat.start_if_configured()

    webui.main()  # blocking - serves the config UI in the foreground


if __name__ == "__main__":
    main()
```

- [ ] **Step 6: Add `RUSTYLOX_URL` to `.env.example`**

Append to `.env.example`:

```
# Optional: base URL of a RustyLox instance to self-register with (e.g.
# http://10.0.0.32:8080). Empty/unset = no registration, addon works fully
# standalone. On startup and every 60s after, this container POSTs its name,
# version, and config API URL to RustyLox's addon registry so its settings
# become editable from RustyLox's own UI.
RUSTYLOX_URL=
```

- [ ] **Step 7: Run the full test suite**

```bash
cd /home/boern/projects/kia-connect-bridge
uv run pytest -v
```

Expected: all tests pass

- [ ] **Step 8: Commit**

```bash
cd /home/boern/projects/kia-connect-bridge
git add heartbeat.py run.py .env.example tests/test_heartbeat.py
git commit -m "feat: add RustyLox self-registration heartbeat"
```

---

### Task 3: OCI labels + GHCR publish workflow

**Files:**
- Modify: `Dockerfile`
- Create: `.github/workflows/ci.yml`

**Interfaces:** none (build/publish only, no runtime code).

- [ ] **Step 1: Add OCI labels to `Dockerfile`**

```dockerfile
FROM python:3.12-alpine

LABEL io.rustylox.addon="true" \
      org.opencontainers.image.title="kia-connect-bridge" \
      org.opencontainers.image.description="Bridges Kia Connect vehicle SOC/range/charging state into MQTT for Loxone" \
      org.opencontainers.image.source="https://github.com/boernmaster/kia-connect-bridge"

COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

WORKDIR /app
COPY pyproject.toml ./
RUN uv pip install --system --no-cache-dir hyundai-kia-connect-api paho-mqtt

COPY kia_soc_bridge.py webui.py addon_api.py heartbeat.py run.py ./

CMD ["python", "run.py"]
```

- [ ] **Step 2: Verify the image still builds locally**

```bash
cd /home/boern/projects/kia-connect-bridge
docker build -t kia-connect-bridge:label-test .
docker inspect kia-connect-bridge:label-test --format '{{ index .Config.Labels "io.rustylox.addon" }}'
```

Expected: `true`

- [ ] **Step 3: Write `.github/workflows/ci.yml`**, modeled on RustyLox's own tag-triggered publish job:

```yaml
name: CI

on:
  push:
    branches: [main]
    tags: ["v*"]
  pull_request:
    branches: [main]

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository_owner }}/kia-connect-bridge

jobs:
  test:
    name: Test
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v3
      - run: uv sync --group dev
      - run: uv run pytest -v

  docker-build:
    name: Build and push Docker image
    runs-on: ubuntu-24.04
    needs: [test]
    if: startsWith(github.ref, 'refs/tags/v')
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4

      - uses: docker/setup-buildx-action@v3

      - uses: docker/login-action@v4
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - uses: docker/metadata-action@v5
        id: meta
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}

      - uses: docker/build-push-action@v7
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
```

- [ ] **Step 4: Commit**

```bash
cd /home/boern/projects/kia-connect-bridge
git add Dockerfile .github/workflows/ci.yml
git commit -m "ci: publish tagged releases to ghcr.io with RustyLox addon labels"
```

- [ ] **Step 5: Tag the first release once merged** (manual, after the plan is done and reviewed)

```bash
git tag v1.0.0
git push origin v1.0.0
```

---

## Track B: RustyLox (platform side)

Repo: `/home/boern/apps/RustyLox`

### Task 4: `addon-registry` crate scaffold + in-memory registry

**Files:**
- Create: `crates/addon-registry/Cargo.toml`
- Create: `crates/addon-registry/src/lib.rs`
- Create: `crates/addon-registry/src/model.rs`
- Create: `crates/addon-registry/src/registry.rs`
- Modify: `Cargo.toml` (workspace members)

**Interfaces:**
- Produces: `addon_registry::AddonInstance { name, version, config_api_base_url, last_seen }`, `addon_registry::Registry::new() -> Self`, `.register(instance: AddonInstance)`, `.list(now: DateTime<Utc>) -> Vec<AddonInstanceView>` (with computed `online: bool`) — consumed by Task 5 (persistence) and Task 6 (routes).

- [ ] **Step 1: Add the crate to the workspace**

Modify root `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/auth",
    "crates/rustylox-core",
    "crates/rustylox-config",
    "crates/rustylox-logging",
    "crates/backup-manager",
    "crates/miniserver-client",
    "crates/plugin-manager",
    "crates/mqtt-gateway",
    "crates/metrics",
    "crates/email-manager",
    "crates/task-scheduler",
    "crates/web-api",
    "crates/web-ui",
    "crates/rustylox-daemon",
    "crates/addon-registry",
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
axum = { version = "0.7", features = ["ws", "macros", "multipart"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
config = "0.14"
dashmap = "6.0"
tempfile = "3.8"
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
```

(Adding `chrono` to workspace deps — not previously listed; needed for `last_seen` timestamps.)

- [ ] **Step 2: Create `crates/addon-registry/Cargo.toml`**

```toml
[package]
name = "addon-registry"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tokio = { workspace = true }
reqwest = { workspace = true }
chrono = { workspace = true }
rustylox-core = { path = "../rustylox-core" }

[dev-dependencies]
axum = { workspace = true }
tempfile = { workspace = true }
```

- [ ] **Step 3: Write the failing test for the in-memory registry** (in `crates/addon-registry/src/registry.rs`, as an inline `#[cfg(test)]` module)

```rust
//! In-memory registry of self-registered addon instances.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::model::{AddonInstance, AddonInstanceView};

/// An instance is considered offline after this many missed heartbeats
/// (heartbeat interval is 60s, so 3 missed = ~180s of silence).
const OFFLINE_AFTER_MISSED_HEARTBEATS: i64 = 3;
const HEARTBEAT_INTERVAL_SECONDS: i64 = 60;

pub struct Registry {
    instances: Arc<Mutex<HashMap<String, AddonInstance>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register (or re-register) an instance. Last-write-wins by name -
    /// handles container restarts / IP changes on the LAN.
    pub async fn register(&self, instance: AddonInstance) {
        let mut guard = self.instances.lock().await;
        guard.insert(instance.name.clone(), instance);
    }

    pub async fn list(&self, now: DateTime<Utc>) -> Vec<AddonInstanceView> {
        let guard = self.instances.lock().await;
        let cutoff = Duration::seconds(HEARTBEAT_INTERVAL_SECONDS * OFFLINE_AFTER_MISSED_HEARTBEATS);
        let mut views: Vec<AddonInstanceView> = guard
            .values()
            .map(|instance| AddonInstanceView {
                name: instance.name.clone(),
                version: instance.version.clone(),
                config_api_base_url: instance.config_api_base_url.clone(),
                online: now.signed_duration_since(instance.last_seen) <= cutoff,
            })
            .collect();
        views.sort_by(|a, b| a.name.cmp(&b.name));
        views
    }

    pub async fn find(&self, name: &str) -> Option<AddonInstance> {
        let guard = self.instances.lock().await;
        guard.get(name).cloned()
    }

    pub(crate) async fn snapshot(&self) -> HashMap<String, AddonInstance> {
        self.instances.lock().await.clone()
    }

    pub(crate) async fn replace_all(&self, instances: HashMap<String, AddonInstance>) {
        let mut guard = self.instances.lock().await;
        *guard = instances;
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str, last_seen: DateTime<Utc>) -> AddonInstance {
        AddonInstance {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            config_api_base_url: "http://10.0.0.32:8090".to_string(),
            last_seen,
        }
    }

    #[tokio::test]
    async fn registered_instance_is_online_immediately() {
        let registry = Registry::new();
        let now = Utc::now();
        registry.register(sample("kia-connect-bridge", now)).await;

        let views = registry.list(now).await;

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "kia-connect-bridge");
        assert!(views[0].online);
    }

    #[tokio::test]
    async fn instance_goes_offline_after_missed_heartbeats() {
        let registry = Registry::new();
        let now = Utc::now();
        let stale = now - Duration::seconds(200); // > 180s cutoff
        registry.register(sample("kia-connect-bridge", stale)).await;

        let views = registry.list(now).await;

        assert!(!views[0].online);
    }

    #[tokio::test]
    async fn re_registering_same_name_is_last_write_wins() {
        let registry = Registry::new();
        let now = Utc::now();
        registry
            .register(sample("kia-connect-bridge", now - Duration::seconds(500)))
            .await;
        registry.register(sample("kia-connect-bridge", now)).await;

        let views = registry.list(now).await;

        assert_eq!(views.len(), 1);
        assert!(views[0].online);
    }

    #[tokio::test]
    async fn list_is_sorted_by_name() {
        let registry = Registry::new();
        let now = Utc::now();
        registry.register(sample("zzz-addon", now)).await;
        registry.register(sample("aaa-addon", now)).await;

        let views = registry.list(now).await;

        assert_eq!(views[0].name, "aaa-addon");
        assert_eq!(views[1].name, "zzz-addon");
    }
}
```

- [ ] **Step 4: Write `crates/addon-registry/src/model.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// What an addon POSTs to /api/addons/register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub version: String,
    pub config_api_base_url: String,
}

/// Stored server-side, adds the last-seen timestamp used for staleness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonInstance {
    pub name: String,
    pub version: String,
    pub config_api_base_url: String,
    pub last_seen: DateTime<Utc>,
}

impl AddonInstance {
    pub fn from_request(request: RegisterRequest, now: DateTime<Utc>) -> Self {
        Self {
            name: request.name,
            version: request.version,
            config_api_base_url: request.config_api_base_url,
            last_seen: now,
        }
    }
}

/// What GET /api/addons returns per instance - never includes raw internals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonInstanceView {
    pub name: String,
    pub version: String,
    pub config_api_base_url: String,
    pub online: bool,
}
```

- [ ] **Step 5: Write `crates/addon-registry/src/lib.rs`**

```rust
pub mod model;
pub mod registry;

pub use model::{AddonInstance, AddonInstanceView, RegisterRequest};
pub use registry::Registry;
```

- [ ] **Step 6: Run the tests**

```bash
cd /home/boern/apps/RustyLox
cargo test -p addon-registry
```

Expected: 4 passed

- [ ] **Step 7: Commit**

```bash
cd /home/boern/apps/RustyLox
git add Cargo.toml crates/addon-registry
git commit -m "feat(addon-registry): scaffold crate with in-memory instance registry"
```

---

### Task 5: JSON file persistence

**Files:**
- Create: `crates/addon-registry/src/persistence.rs`
- Modify: `crates/addon-registry/src/lib.rs`
- Modify: `crates/addon-registry/src/registry.rs`

**Interfaces:**
- Consumes: `Registry::snapshot()`/`replace_all()` from Task 4.
- Produces: `Registry::load(path: &Path) -> Result<Self>`, `Registry::save(&self, path: &Path) -> Result<()>` — consumed by Task 6, which loads at daemon startup and saves after every register call.

- [ ] **Step 1: Write the failing test** (append to `crates/addon-registry/src/registry.rs`'s test module)

```rust
    #[tokio::test]
    async fn save_then_load_round_trips_instances() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let path = dir.path().join("addonregistry.json");
        let now = Utc::now();

        let registry = Registry::new();
        registry.register(sample("kia-connect-bridge", now)).await;
        registry.save(&path).await.expect("save should succeed");

        let loaded = Registry::load(&path).await.expect("load should succeed");
        let views = loaded.list(now).await;

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "kia-connect-bridge");
    }

    #[tokio::test]
    async fn load_returns_empty_registry_when_file_missing() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let path = dir.path().join("does-not-exist.json");

        let registry = Registry::load(&path).await.expect("load should succeed");
        let views = registry.list(Utc::now()).await;

        assert!(views.is_empty());
    }
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd /home/boern/apps/RustyLox
cargo test -p addon-registry save_then_load
```

Expected: compile error, `no method named 'save'/'load' found`

- [ ] **Step 3: Write `crates/addon-registry/src/persistence.rs`**, mirroring `crates/plugin-manager/src/database.rs`'s load/save-via-tmp-then-rename pattern:

```rust
use std::collections::HashMap;
use std::path::Path;

use rustylox_core::{Error, Result};
use tokio::fs;

use crate::model::AddonInstance;

pub async fn load(path: &Path) -> Result<HashMap<String, AddonInstance>> {
    if !path.exists() {
        tracing::info!(
            "Addon registry file not found, starting empty: {}",
            path.display()
        );
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| Error::plugin(format!("Failed to read addon registry: {}", e)))?;
    let instances: HashMap<String, AddonInstance> = serde_json::from_str(&content)
        .map_err(|e| Error::plugin(format!("Failed to parse addon registry: {}", e)))?;
    Ok(instances)
}

pub async fn save(path: &Path, instances: &HashMap<String, AddonInstance>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::plugin(format!("Failed to create addon registry dir: {}", e)))?;
    }
    let content = serde_json::to_string_pretty(instances)
        .map_err(|e| Error::plugin(format!("Failed to serialize addon registry: {}", e)))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &content)
        .await
        .map_err(|e| Error::plugin(format!("Failed to write addon registry: {}", e)))?;
    fs::rename(&tmp, path)
        .await
        .map_err(|e| Error::plugin(format!("Failed to finalize addon registry write: {}", e)))?;
    Ok(())
}
```

- [ ] **Step 4: Add `load`/`save` methods to `Registry`** in `crates/addon-registry/src/registry.rs`:

```rust
use std::path::Path;

impl Registry {
    pub async fn load(path: &Path) -> rustylox_core::Result<Self> {
        let instances = crate::persistence::load(path).await?;
        Ok(Self {
            instances: Arc::new(Mutex::new(instances)),
        })
    }

    pub async fn save(&self, path: &Path) -> rustylox_core::Result<()> {
        let snapshot = self.snapshot().await;
        crate::persistence::save(path, &snapshot).await
    }
}
```

- [ ] **Step 5: Add the module to `crates/addon-registry/src/lib.rs`**

```rust
pub mod model;
pub mod persistence;
pub mod registry;

pub use model::{AddonInstance, AddonInstanceView, RegisterRequest};
pub use registry::Registry;
```

- [ ] **Step 6: Run the tests**

```bash
cd /home/boern/apps/RustyLox
cargo test -p addon-registry
```

Expected: 6 passed

- [ ] **Step 7: Commit**

```bash
cd /home/boern/apps/RustyLox
git add crates/addon-registry
git commit -m "feat(addon-registry): persist registered instances to JSON"
```

---

### Task 6: `POST /api/addons/register` and `GET /api/addons` routes

**Files:**
- Modify: `crates/web-api/Cargo.toml`
- Modify: `crates/web-api/src/state.rs`
- Create: `crates/web-api/src/routes/addons.rs`
- Modify: `crates/web-api/src/routes/mod.rs`
- Modify: `crates/web-api/src/lib.rs`
- Create: `crates/web-api/tests/addons_registry.rs`
- Modify: `crates/rustylox-daemon/src/main.rs`

**Interfaces:**
- Consumes: `addon_registry::{Registry, RegisterRequest, AddonInstance}` from Tasks 4/5.
- Produces: `AppState.addon_registry: Option<Arc<Registry>>`, routes `POST /api/addons/register` (201/400) and `GET /api/addons` (200, `Vec<AddonInstanceView>` JSON) — consumed by Task 7 (proxy routes reuse the same state field) and Task 10 (web-ui reads via an internal call, not HTTP, see Task 10).

- [ ] **Step 1: Add the path dependency**

Modify `crates/web-api/Cargo.toml`, adding under `[dependencies]`:

```toml
addon-registry = { path = "../addon-registry" }
```

- [ ] **Step 2: Add `addon_registry` to `AppState`**

Modify `crates/web-api/src/state.rs` — add the field (as `Option<Arc<...>>`, following the exact `auth_service`/`weather_service` optional-service pattern already in this struct) and a builder method:

```rust
use addon_registry::Registry as AddonRegistry;
```

Add to the `AppState` struct (after `weather_service`):

```rust
    /// Addon registry (self-registered containerized addons) - optional,
    /// only present once the daemon has loaded it from disk at startup.
    pub addon_registry: Option<Arc<AddonRegistry>>,
```

Add to the `Self { ... }` literal in `new_with_shared_config` (after `weather_service: None,`):

```rust
            addon_registry: None,
```

Add a builder method (after `with_weather`):

```rust
    /// Attach an AddonRegistry to the application state
    pub fn with_addon_registry(mut self, addon_registry: Arc<AddonRegistry>) -> Self {
        self.addon_registry = Some(addon_registry);
        self
    }
```

- [ ] **Step 3: Write the failing integration test**

Create `crates/web-api/tests/addons_registry.rs`, following the exact `test_state()` construction already used in `crates/web-api/tests/virtual_input_test.rs` (real, confirmed pattern — `ConfigManager::new(&config_dir)` takes a path, not an in-memory constructor):

```rust
use addon_registry::Registry;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::path::PathBuf;
use std::sync::Arc;
use tower::util::ServiceExt;
use web_api::{create_router, AppState};

fn test_state(registry: Arc<Registry>) -> AppState {
    let tmp = PathBuf::from("/tmp/rustylox-test-addons");
    let config_dir = tmp.join("config/system");
    std::fs::create_dir_all(&config_dir).ok();

    let config_manager = rustylox_config::ConfigManager::new(&config_dir);
    let config = rustylox_config::GeneralConfig::default();

    AppState::new(tmp, "test".to_string(), config_manager, config, None)
        .with_addon_registry(registry)
}

/// Helper: read a response body to a String (matches virtual_input_test.rs's convention).
async fn body_string(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

#[tokio::test]
async fn register_then_list_round_trips() {
    let registry = Arc::new(Registry::new());
    let app = create_router(test_state(registry));

    let register_body = serde_json::json!({
        "name": "kia-connect-bridge",
        "version": "1.0.0",
        "config_api_base_url": "http://10.0.0.32:8090"
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/addons/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/addons")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response).await;
    let instances: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0]["name"], "kia-connect-bridge");
    assert_eq!(instances[0]["online"], true);
}

#[tokio::test]
async fn register_rejects_malformed_payload() {
    let registry = Arc::new(Registry::new());
    let app = create_router(test_state(registry));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/addons/register")
                .header("content-type", "application/json")
                .body(Body::from("{\"name\": \"missing fields\"}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 4: Run to verify it fails**

```bash
cd /home/boern/apps/RustyLox
cargo test -p web-api addons_registry
```

Expected: compile error, route `/api/addons/register` not found (404) or handler module missing

- [ ] **Step 5: Write `crates/web-api/src/routes/addons.rs`**

```rust
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;

use addon_registry::{AddonInstance, RegisterRequest};
use tracing::{error, warn};

use crate::state::AppState;

/// POST /api/addons/register
pub async fn register(
    State(state): State<AppState>,
    body: Result<Json<RegisterRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let Json(request) = match body {
        Ok(json) => json,
        Err(e) => {
            warn!("Rejected malformed addon registration: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Invalid registration payload: {}", e) })),
            )
                .into_response();
        }
    };

    if request.name.trim().is_empty() || request.config_api_base_url.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "name and config_api_base_url are required" })),
        )
            .into_response();
    }

    let Some(registry) = &state.addon_registry else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Addon registry not configured" })),
        )
            .into_response();
    };

    let instance = AddonInstance::from_request(request, Utc::now());
    registry.register(instance).await;

    if let Some(path) = &state.addon_registry_path {
        if let Err(e) = registry.save(path).await {
            error!("Failed to persist addon registry: {}", e);
        }
    }

    StatusCode::CREATED.into_response()
}

/// GET /api/addons
pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    let Some(registry) = &state.addon_registry else {
        return Json(Vec::<addon_registry::AddonInstanceView>::new()).into_response();
    };
    let views = registry.list(Utc::now()).await;
    Json(views).into_response()
}
```

- [ ] **Step 6: Add `addon_registry_path` to `AppState`**

Modify `crates/web-api/src/state.rs` — the handler above needs to know where to save after every register call. Add alongside `addon_registry`:

```rust
    /// Path to the addon registry JSON file (data/system/addonregistry.json
    /// under lbhomedir) - set whenever addon_registry is attached.
    pub addon_registry_path: Option<std::path::PathBuf>,
```

Add to the `Self { ... }` literal:

```rust
            addon_registry_path: None,
```

Update the builder method to set both fields together:

```rust
    /// Attach an AddonRegistry to the application state
    pub fn with_addon_registry(
        mut self,
        addon_registry: Arc<AddonRegistry>,
        addon_registry_path: std::path::PathBuf,
    ) -> Self {
        self.addon_registry = Some(addon_registry);
        self.addon_registry_path = Some(addon_registry_path);
        self
    }
```

Update the test helper in `crates/web-api/tests/addons_registry.rs` accordingly (using the same `tmp` path the rest of `test_state()` already builds from):

```rust
fn test_state(registry: Arc<Registry>) -> AppState {
    let tmp = PathBuf::from("/tmp/rustylox-test-addons");
    let config_dir = tmp.join("config/system");
    std::fs::create_dir_all(&config_dir).ok();

    let config_manager = rustylox_config::ConfigManager::new(&config_dir);
    let config = rustylox_config::GeneralConfig::default();

    AppState::new(tmp.clone(), "test".to_string(), config_manager, config, None)
        .with_addon_registry(registry, tmp.join("data/system/addonregistry.json"))
}
```

- [ ] **Step 7: Register the module and routes**

Modify `crates/web-api/src/routes/mod.rs` — add:

```rust
pub mod addons;
```

Modify `crates/web-api/src/lib.rs` — add a `// Addon routes` section to the `Router::new()` chain in `create_router`, next to the existing `// Plugin routes` section:

```rust
        // Addon routes
        .route("/api/addons/register", post(routes::addons::register))
        .route("/api/addons", get(routes::addons::list))
```

- [ ] **Step 8: Run the tests**

```bash
cd /home/boern/apps/RustyLox
cargo test -p web-api addons_registry
```

Expected: 2 passed

- [ ] **Step 9: Wire real startup loading in the daemon**

Modify `crates/rustylox-daemon/src/main.rs` — near where `lbhomedir` is read (per the earlier finding at line ~26-28), load the registry and attach it to `AppState`:

```rust
    let addon_registry_path = lbhomedir.join("data/system/addonregistry.json");
    let addon_registry = std::sync::Arc::new(
        addon_registry::Registry::load(&addon_registry_path)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to load addon registry, starting empty: {}", e);
                addon_registry::Registry::new()
            }),
    );
```

Confirmed real call site — `crates/rustylox-daemon/src/main.rs:160-168` builds `AppState` as one chained expression:

```rust
    let state = AppState::new_with_shared_config(
        lbhomedir,
        version.to_string(),
        config_manager,
        config,
        mqtt_gateway,
    )
    .with_log_level_updater(log_level_updater)
    .with_auth(auth_service)
    .with_weather(Arc::clone(&weather_service));
```

Add `.with_addon_registry(addon_registry, addon_registry_path)` to this same chain (the `addon_registry`/`addon_registry_path` bindings from this step must be created *before* this `let state = ...` statement, since `lbhomedir` is moved into `AppState::new_with_shared_config` on the same line — read `lbhomedir.join(...)` before that call, not after):

```rust
    let state = AppState::new_with_shared_config(
        lbhomedir.clone(),
        version.to_string(),
        config_manager,
        config,
        mqtt_gateway,
    )
    .with_log_level_updater(log_level_updater)
    .with_auth(auth_service)
    .with_weather(Arc::clone(&weather_service))
    .with_addon_registry(addon_registry, addon_registry_path);
```

(Note the added `.clone()` on `lbhomedir` — the addon registry path is derived from it just above and `AppState::new_with_shared_config` otherwise consumes it by value.)

- [ ] **Step 10: Full workspace build**

```bash
cd /home/boern/apps/RustyLox
cargo build --workspace
```

Expected: builds clean

- [ ] **Step 11: Commit**

```bash
cd /home/boern/apps/RustyLox
git add crates/web-api crates/rustylox-daemon crates/addon-registry
git commit -m "feat(web-api): add addon registration and listing routes"
```

---

### Task 7: Config proxy (`GET/POST /api/addons/:name/{schema,config}`)

**Files:**
- Create: `crates/addon-registry/src/proxy.rs`
- Modify: `crates/addon-registry/src/lib.rs`
- Modify: `crates/web-api/src/routes/addons.rs`
- Modify: `crates/web-api/src/lib.rs`

**Interfaces:**
- Consumes: `Registry::find(name) -> Option<AddonInstance>` from Task 4.
- Produces: `addon_registry::proxy::{fetch_schema, fetch_config, save_config}(base_url: &str) -> Result<serde_json::Value, ProxyError>` — consumed by the new web-api routes, and later by Task 10's web-ui handler.

- [ ] **Step 1: Write the failing test** (in `crates/addon-registry/src/proxy.rs`, inline `#[cfg(test)]`, spinning a fake addon server with axum since there's no existing mock-server convention in this codebase)

```rust
//! Forwards schema/config calls to a registered addon instance's own
//! config API. An addon being unreachable must never crash or hang RustyLox -
//! every call here returns a typed error the caller turns into a clean
//! "addon offline" response.

use serde_json::Value;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("addon unreachable: {0}")]
    Unreachable(String),
    #[error("addon returned an error status: {0}")]
    BadStatus(reqwest::StatusCode),
    #[error("failed to parse addon response: {0}")]
    InvalidResponse(String),
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build addon proxy HTTP client")
}

pub async fn fetch_schema(base_url: &str) -> Result<Value, ProxyError> {
    get_json(base_url, "/addon/schema").await
}

pub async fn fetch_config(base_url: &str) -> Result<Value, ProxyError> {
    get_json(base_url, "/addon/config").await
}

async fn get_json(base_url: &str, path: &str) -> Result<Value, ProxyError> {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let response = client()
        .get(&url)
        .send()
        .await
        .map_err(|e| ProxyError::Unreachable(e.to_string()))?;
    if !response.status().is_success() {
        return Err(ProxyError::BadStatus(response.status()));
    }
    response
        .json::<Value>()
        .await
        .map_err(|e| ProxyError::InvalidResponse(e.to_string()))
}

pub async fn save_config(base_url: &str, payload: &Value) -> Result<(), ProxyError> {
    let url = format!("{}/addon/config", base_url.trim_end_matches('/'));
    let response = client()
        .post(&url)
        .json(payload)
        .send()
        .await
        .map_err(|e| ProxyError::Unreachable(e.to_string()))?;
    if !response.status().is_success() {
        return Err(ProxyError::BadStatus(response.status()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    async fn spawn_fake_addon() -> String {
        async fn schema() -> Json<Value> {
            Json(json!([{"key": "MQTT_HOST", "label": "MQTT Host", "type": "text", "help": "", "secret": false}]))
        }
        async fn config() -> Json<Value> {
            Json(json!({"MQTT_HOST": {"value": "10.0.0.32", "secret_set": false}}))
        }
        async fn save(Json(_body): Json<Value>) -> Json<Value> {
            Json(json!({"saved": true}))
        }

        let app = Router::new()
            .route("/addon/schema", get(schema))
            .route("/addon/config", get(config).post(save));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn fetch_schema_returns_addon_fields() {
        let base_url = spawn_fake_addon().await;

        let schema = fetch_schema(&base_url).await.expect("should succeed");

        assert_eq!(schema[0]["key"], "MQTT_HOST");
    }

    #[tokio::test]
    async fn fetch_config_returns_current_values() {
        let base_url = spawn_fake_addon().await;

        let config = fetch_config(&base_url).await.expect("should succeed");

        assert_eq!(config["MQTT_HOST"]["value"], "10.0.0.32");
    }

    #[tokio::test]
    async fn save_config_posts_and_succeeds() {
        let base_url = spawn_fake_addon().await;

        let result = save_config(&base_url, &json!({"MQTT_HOST": "10.0.0.99"})).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fetch_schema_returns_unreachable_for_dead_address() {
        let result = fetch_schema("http://127.0.0.1:1").await;

        assert!(matches!(result, Err(ProxyError::Unreachable(_))));
    }
}
```

- [ ] **Step 2: Add the module to `crates/addon-registry/src/lib.rs`**

```rust
pub mod model;
pub mod persistence;
pub mod proxy;
pub mod registry;

pub use model::{AddonInstance, AddonInstanceView, RegisterRequest};
pub use registry::Registry;
```

- [ ] **Step 3: Run the tests**

```bash
cd /home/boern/apps/RustyLox
cargo test -p addon-registry proxy
```

Expected: 4 passed

- [ ] **Step 4: Add the web-api routes**

Modify `crates/web-api/src/routes/addons.rs`, appending:

```rust
use axum::extract::Path;
use addon_registry::proxy;

/// GET /api/addons/:name/schema
pub async fn schema(State(state): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    proxy_get(&state, &name, proxy::fetch_schema).await
}

/// GET /api/addons/:name/config
pub async fn config(State(state): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    proxy_get(&state, &name, proxy::fetch_config).await
}

async fn proxy_get<F, Fut>(state: &AppState, name: &str, call: F) -> axum::response::Response
where
    F: FnOnce(&str) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, proxy::ProxyError>>,
{
    let Some(registry) = &state.addon_registry else {
        return (StatusCode::SERVICE_UNAVAILABLE, "Addon registry not configured").into_response();
    };
    let Some(instance) = registry.find(name).await else {
        return (StatusCode::NOT_FOUND, format!("Unknown addon: {}", name)).into_response();
    };
    match call(&instance.config_api_base_url).await {
        Ok(value) => Json(value).into_response(),
        Err(e) => {
            warn!("Proxy call to addon '{}' failed: {}", name, e);
            (StatusCode::BAD_GATEWAY, Json(serde_json::json!({ "error": "addon offline" }))).into_response()
        }
    }
}

/// POST /api/addons/:name/config
pub async fn save_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let Some(registry) = &state.addon_registry else {
        return (StatusCode::SERVICE_UNAVAILABLE, "Addon registry not configured").into_response();
    };
    let Some(instance) = registry.find(&name).await else {
        return (StatusCode::NOT_FOUND, format!("Unknown addon: {}", name)).into_response();
    };
    match proxy::save_config(&instance.config_api_base_url, &payload).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            warn!("Proxy save to addon '{}' failed: {}", name, e);
            (StatusCode::BAD_GATEWAY, Json(serde_json::json!({ "error": "addon offline" }))).into_response()
        }
    }
}
```

- [ ] **Step 5: Register the routes**

Modify `crates/web-api/src/lib.rs`, extending the `// Addon routes` section:

```rust
        // Addon routes
        .route("/api/addons/register", post(routes::addons::register))
        .route("/api/addons", get(routes::addons::list))
        .route("/api/addons/:name/schema", get(routes::addons::schema))
        .route(
            "/api/addons/:name/config",
            get(routes::addons::config).post(routes::addons::save_config),
        )
```

- [ ] **Step 6: Write an integration test for the offline case**

Append to `crates/web-api/tests/addons_registry.rs`:

```rust
#[tokio::test]
async fn proxy_returns_bad_gateway_when_addon_offline() {
    let registry = Arc::new(Registry::new());
    registry
        .register(addon_registry::AddonInstance {
            name: "dead-addon".to_string(),
            version: "1.0.0".to_string(),
            config_api_base_url: "http://127.0.0.1:1".to_string(),
            last_seen: chrono::Utc::now(),
        })
        .await;
    let app = create_router(test_state(registry));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/addons/dead-addon/schema")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

// (test_state and body_string are already defined earlier in this file, see Task 6.)

#[tokio::test]
async fn proxy_returns_not_found_for_unknown_addon() {
    let registry = Arc::new(Registry::new());
    let app = create_router(test_state(registry));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/addons/nonexistent/schema")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 7: Run all web-api and addon-registry tests**

```bash
cd /home/boern/apps/RustyLox
cargo test -p addon-registry -p web-api
```

Expected: all passed

- [ ] **Step 8: Commit**

```bash
cd /home/boern/apps/RustyLox
git add crates/addon-registry crates/web-api
git commit -m "feat: proxy addon schema/config calls through the registry"
```

---

### Task 8: GHCR catalog client

**Files:**
- Create: `crates/addon-registry/src/catalog.rs`
- Modify: `crates/addon-registry/src/lib.rs`

**Interfaces:**
- Produces: `addon_registry::catalog::CatalogClient::new(user: String, token: String) -> Self`, `.with_base_urls(github_api: String, ghcr: String) -> Self` (test hook), `.list_addons(&self) -> Result<Vec<CatalogEntry>, CatalogError>` — consumed by Task 9's route.

- [ ] **Step 1: Write the failing test**, using fake local servers standing in for `api.github.com` and `ghcr.io` (same axum-test-server approach as Task 7, since these are real external hosts that must be swappable for tests):

```rust
//! Builds the addon discovery catalog from GitHub Container Registry:
//! lists packages under a GitHub user via the Packages API, then reads each
//! image's OCI labels via the GHCR distribution API to find ones marked
//! io.rustylox.addon=true.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub name: String,
    pub title: String,
    pub description: String,
    pub source: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("GitHub API request failed: {0}")]
    RequestFailed(String),
    #[error("unexpected response shape: {0}")]
    InvalidResponse(String),
}

const CACHE_TTL: Duration = Duration::from_secs(600); // 10 minutes

struct CachedCatalog {
    entries: Vec<CatalogEntry>,
    fetched_at: Instant,
}

pub struct CatalogClient {
    user: String,
    token: String,
    github_api_base: String,
    ghcr_base: String,
    cache: Mutex<Option<CachedCatalog>>,
}

impl CatalogClient {
    pub fn new(user: String, token: String) -> Self {
        Self {
            user,
            token,
            github_api_base: "https://api.github.com".to_string(),
            ghcr_base: "https://ghcr.io".to_string(),
            cache: Mutex::new(None),
        }
    }

    /// Test hook: point at fake local servers instead of the real hosts.
    pub fn with_base_urls(mut self, github_api_base: String, ghcr_base: String) -> Self {
        self.github_api_base = github_api_base;
        self.ghcr_base = ghcr_base;
        self
    }

    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build catalog HTTP client")
    }

    pub async fn list_addons(&self) -> Result<Vec<CatalogEntry>, CatalogError> {
        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.as_ref() {
                if cached.fetched_at.elapsed() < CACHE_TTL {
                    return Ok(cached.entries.clone());
                }
            }
        }

        let packages = self.list_packages().await?;
        let mut entries = Vec::new();
        for package_name in packages {
            if let Some(entry) = self.fetch_labels(&package_name).await? {
                entries.push(entry);
            }
        }

        let mut cache = self.cache.lock().await;
        *cache = Some(CachedCatalog {
            entries: entries.clone(),
            fetched_at: Instant::now(),
        });
        Ok(entries)
    }

    async fn list_packages(&self) -> Result<Vec<String>, CatalogError> {
        let url = format!(
            "{}/users/{}/packages?package_type=container",
            self.github_api_base, self.user
        );
        let response = self
            .client()
            .get(&url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "rustylox-addon-catalog")
            .send()
            .await
            .map_err(|e| CatalogError::RequestFailed(e.to_string()))?;
        if !response.status().is_success() {
            return Err(CatalogError::RequestFailed(format!(
                "GitHub Packages API returned {}",
                response.status()
            )));
        }
        let body: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| CatalogError::InvalidResponse(e.to_string()))?;
        Ok(body
            .into_iter()
            .filter_map(|entry| entry.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect())
    }

    async fn fetch_labels(&self, package_name: &str) -> Result<Option<CatalogEntry>, CatalogError> {
        let token_url = format!(
            "{}/token?service=ghcr.io&scope=repository:{}/{}:pull",
            self.ghcr_base, self.user, package_name
        );
        let token_response = self
            .client()
            .get(&token_url)
            .basic_auth(&self.user, Some(&self.token))
            .send()
            .await
            .map_err(|e| CatalogError::RequestFailed(e.to_string()))?;
        let token_body: serde_json::Value = token_response
            .json()
            .await
            .map_err(|e| CatalogError::InvalidResponse(e.to_string()))?;
        let ghcr_token = token_body
            .get("token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| CatalogError::InvalidResponse("missing token in GHCR response".to_string()))?;

        let manifest_url = format!("{}/v2/{}/{}/manifests/latest", self.ghcr_base, self.user, package_name);
        let manifest_response = self
            .client()
            .get(&manifest_url)
            .bearer_auth(ghcr_token)
            .header("Accept", "application/vnd.oci.image.manifest.v1+json")
            .send()
            .await
            .map_err(|e| CatalogError::RequestFailed(e.to_string()))?;
        let manifest: serde_json::Value = manifest_response
            .json()
            .await
            .map_err(|e| CatalogError::InvalidResponse(e.to_string()))?;
        let digest = manifest
            .get("config")
            .and_then(|c| c.get("digest"))
            .and_then(|d| d.as_str())
            .ok_or_else(|| CatalogError::InvalidResponse("missing config digest in manifest".to_string()))?;

        let blob_url = format!("{}/v2/{}/{}/blobs/{}", self.ghcr_base, self.user, package_name, digest);
        let blob_response = self
            .client()
            .get(&blob_url)
            .bearer_auth(ghcr_token)
            .send()
            .await
            .map_err(|e| CatalogError::RequestFailed(e.to_string()))?;
        let blob: serde_json::Value = blob_response
            .json()
            .await
            .map_err(|e| CatalogError::InvalidResponse(e.to_string()))?;

        let labels = blob.get("config").and_then(|c| c.get("Labels"));
        let is_addon = labels
            .and_then(|l| l.get("io.rustylox.addon"))
            .and_then(|v| v.as_str())
            .map(|v| v == "true")
            .unwrap_or(false);
        if !is_addon {
            return Ok(None);
        }

        let get_label = |key: &str, default: &str| {
            labels
                .and_then(|l| l.get(key))
                .and_then(|v| v.as_str())
                .unwrap_or(default)
                .to_string()
        };

        Ok(Some(CatalogEntry {
            name: package_name.to_string(),
            title: get_label("org.opencontainers.image.title", package_name),
            description: get_label("org.opencontainers.image.description", ""),
            source: get_label("org.opencontainers.image.source", ""),
        }))
    }
}
```

- [ ] **Step 2: Write the test module** (append to `catalog.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Path;
    use axum::routing::get;
    use axum::{Json, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    async fn spawn_fake_github_and_ghcr() -> (String, String) {
        async fn packages() -> Json<serde_json::Value> {
            Json(json!([{"name": "kia-connect-bridge"}, {"name": "unrelated-image"}]))
        }
        async fn token() -> Json<serde_json::Value> {
            Json(json!({"token": "fake-token"}))
        }
        async fn manifest(Path((_user, name)): Path<(String, String)>) -> Json<serde_json::Value> {
            Json(json!({"config": {"digest": format!("sha256:fake-{}", name)}}))
        }
        async fn blob(Path((_user, name, _digest)): Path<(String, String, String)>) -> Json<serde_json::Value> {
            if name == "kia-connect-bridge" {
                Json(json!({"config": {"Labels": {
                    "io.rustylox.addon": "true",
                    "org.opencontainers.image.title": "kia-connect-bridge",
                    "org.opencontainers.image.description": "Bridges Kia Connect into MQTT",
                    "org.opencontainers.image.source": "https://github.com/boernmaster/kia-connect-bridge"
                }}}))
            } else {
                Json(json!({"config": {"Labels": {}}}))
            }
        }

        let github_app = Router::new().route("/users/:user/packages", get(packages));
        let github_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let github_addr = github_listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(github_listener, github_app).await.unwrap();
        });

        let ghcr_app = Router::new()
            .route("/token", get(token))
            .route("/v2/:user/:name/manifests/latest", get(manifest))
            .route("/v2/:user/:name/blobs/:digest", get(blob));
        let ghcr_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ghcr_addr = ghcr_listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(ghcr_listener, ghcr_app).await.unwrap();
        });

        (
            format!("http://{}", github_addr),
            format!("http://{}", ghcr_addr),
        )
    }

    #[tokio::test]
    async fn list_addons_filters_to_labeled_images_only() {
        let (github_base, ghcr_base) = spawn_fake_github_and_ghcr().await;
        let client = CatalogClient::new("boernmaster".to_string(), "fake-pat".to_string())
            .with_base_urls(github_base, ghcr_base);

        let entries = client.list_addons().await.expect("should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "kia-connect-bridge");
        assert_eq!(entries[0].title, "kia-connect-bridge");
        assert_eq!(entries[0].source, "https://github.com/boernmaster/kia-connect-bridge");
    }

    #[tokio::test]
    async fn list_addons_caches_within_ttl() {
        let (github_base, ghcr_base) = spawn_fake_github_and_ghcr().await;
        let client = CatalogClient::new("boernmaster".to_string(), "fake-pat".to_string())
            .with_base_urls(github_base, ghcr_base);

        let first = client.list_addons().await.expect("should succeed");
        let second = client.list_addons().await.expect("should succeed");

        assert_eq!(first.len(), second.len());
    }
}
```

- [ ] **Step 3: Add the module to `crates/addon-registry/src/lib.rs`**

```rust
pub mod catalog;
pub mod model;
pub mod persistence;
pub mod proxy;
pub mod registry;

pub use catalog::{CatalogClient, CatalogEntry};
pub use model::{AddonInstance, AddonInstanceView, RegisterRequest};
pub use registry::Registry;
```

- [ ] **Step 4: Run the tests**

```bash
cd /home/boern/apps/RustyLox
cargo test -p addon-registry catalog
```

Expected: 2 passed

- [ ] **Step 5: Commit**

```bash
cd /home/boern/apps/RustyLox
git add crates/addon-registry
git commit -m "feat(addon-registry): add GHCR catalog client filtered by addon label"
```

---

### Task 9: `GET /api/addons/catalog` route + config

**Files:**
- Modify: `crates/web-api/src/state.rs`
- Modify: `crates/web-api/src/routes/addons.rs`
- Modify: `crates/web-api/src/lib.rs`
- Modify: `crates/rustylox-daemon/src/main.rs`
- Modify: `.env.example`

**Interfaces:**
- Consumes: `addon_registry::CatalogClient` from Task 8.
- Produces: `AppState.catalog_client: Option<Arc<CatalogClient>>`, route `GET /api/addons/catalog` — consumed by Task 10's web-ui.

- [ ] **Step 1: Add `catalog_client` to `AppState`**

Modify `crates/web-api/src/state.rs` — add alongside `addon_registry`:

```rust
    /// GHCR-backed addon discovery catalog - None if GITHUB_PACKAGES_TOKEN
    /// isn't configured, in which case the catalog tab just says so.
    pub catalog_client: Option<Arc<addon_registry::CatalogClient>>,
```

Add to the `Self { ... }` literal:

```rust
            catalog_client: None,
```

Add a builder method:

```rust
    /// Attach a CatalogClient to the application state
    pub fn with_catalog_client(mut self, catalog_client: Arc<addon_registry::CatalogClient>) -> Self {
        self.catalog_client = Some(catalog_client);
        self
    }
```

- [ ] **Step 2: Add the route handler**

Modify `crates/web-api/src/routes/addons.rs`, appending:

```rust
/// GET /api/addons/catalog
pub async fn catalog(State(state): State<AppState>) -> impl IntoResponse {
    let Some(client) = &state.catalog_client else {
        return (
            StatusCode::OK,
            Json(serde_json::json!({ "configured": false, "entries": [] })),
        )
            .into_response();
    };
    match client.list_addons().await {
        Ok(entries) => Json(serde_json::json!({ "configured": true, "entries": entries })).into_response(),
        Err(e) => {
            warn!("Catalog fetch failed: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "configured": true, "entries": [], "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
```

- [ ] **Step 3: Register the route**

Modify `crates/web-api/src/lib.rs`, extending the addon routes section:

```rust
        .route("/api/addons/catalog", get(routes::addons::catalog))
```

- [ ] **Step 4: Wire startup config in the daemon**

Modify `crates/rustylox-daemon/src/main.rs`, near the addon registry loading added in Task 6:

```rust
    let catalog_client = match std::env::var("GITHUB_PACKAGES_TOKEN") {
        Ok(token) if !token.trim().is_empty() => {
            let user = std::env::var("GITHUB_PACKAGES_USER").unwrap_or_else(|_| "boernmaster".to_string());
            Some(std::sync::Arc::new(addon_registry::CatalogClient::new(user, token)))
        }
        _ => {
            tracing::info!("GITHUB_PACKAGES_TOKEN not set - addon catalog disabled");
            None
        }
    };
```

`catalog_client` is genuinely optional (unlike `auth_service`/`weather_service`, which this file always constructs unconditionally before the `AppState` chain), so it needs its own conditional attachment. Change the `let state = ...` statement from Task 6 (`crates/rustylox-daemon/src/main.rs:161-171`, already updated with `.with_addon_registry(...)`) from a `let` to a `let mut`, drop the trailing `;` from the chain, and attach the catalog client afterward:

```rust
    let mut state = AppState::new_with_shared_config(
        lbhomedir.clone(),
        version.to_string(),
        config_manager,
        config,
        mqtt_gateway,
    )
    .with_log_level_updater(log_level_updater)
    .with_auth(auth_service)
    .with_weather(Arc::clone(&weather_service))
    .with_addon_registry(addon_registry, addon_registry_path);

    if let Some(client) = catalog_client {
        state = state.with_catalog_client(client);
    }
```

- [ ] **Step 5: Document the new env vars**

Append to `.env.example`:

```
# Optional: enables the Addons catalog tab (GHCR-backed addon discovery).
# GITHUB_PACKAGES_TOKEN needs only the read:packages scope - it cannot push,
# delete, or modify anything. Unset = catalog tab reports "not configured",
# nothing else in the addon system depends on it.
GITHUB_PACKAGES_TOKEN=
GITHUB_PACKAGES_USER=boernmaster
```

- [ ] **Step 6: Build and run the full workspace test suite**

```bash
cd /home/boern/apps/RustyLox
cargo build --workspace
cargo test --workspace
```

Expected: builds and all tests pass

- [ ] **Step 7: Commit**

```bash
cd /home/boern/apps/RustyLox
git add crates/web-api crates/rustylox-daemon .env.example
git commit -m "feat(web-api): expose GHCR addon catalog via /api/addons/catalog"
```

---

### Task 10: web-ui "Addons" page (Catalog + Installed tabs)

**Files:**
- Modify: `crates/web-ui/Cargo.toml`
- Modify: `crates/web-ui/src/templates.rs`
- Create: `crates/web-ui/templates/addons/list.html`
- Create: `crates/web-ui/src/handlers/addons.rs`
- Modify: `crates/web-ui/src/handlers/mod.rs`
- Modify: `crates/web-ui/src/lib.rs`
- Modify: `crates/web-ui/templates/plugins/list.html` (nav link only)

**Interfaces:**
- Consumes: `AppState.addon_registry`/`catalog_client` (Tasks 6/9) directly — web-ui runs in the same process as web-api, so it calls `Registry::list()`/`CatalogClient::list_addons()` in-process rather than over HTTP.

- [ ] **Step 1: Add the path dependency**

Modify `crates/web-ui/Cargo.toml`, adding under `[dependencies]`:

```toml
addon-registry = { path = "../addon-registry" }
```

- [ ] **Step 2: Add template structs**

Modify `crates/web-ui/src/templates.rs`, adding after the existing `PluginListTemplate`/`PluginDisplay` structs:

```rust
#[derive(Template)]
#[template(path = "addons/list.html")]
pub struct AddonsTemplate {
    pub installed: Vec<InstalledAddonDisplay>,
    pub catalog_configured: bool,
    pub catalog_entries: Vec<CatalogEntryDisplay>,
    pub version: String,
    pub lang: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledAddonDisplay {
    pub name: String,
    pub addon_version: String,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntryDisplay {
    pub name: String,
    pub title: String,
    pub description: String,
    pub source: String,
    pub deploy_snippet: String,
}
```

- [ ] **Step 3: Write the handler**

Create `crates/web-ui/src/handlers/addons.rs`, following `handlers/plugins.rs`'s `list` handler structure exactly (data fetch → map to display structs → read lang → render):

```rust
use axum::extract::State;
use axum::response::Html;
use chrono::Utc;

use crate::state::AppState;
use crate::templates::{AddonsTemplate, CatalogEntryDisplay, InstalledAddonDisplay};

pub async fn list(State(state): State<AppState>) -> Html<String> {
    let installed = match &state.addon_registry {
        Some(registry) => registry
            .list(Utc::now())
            .await
            .into_iter()
            .map(|view| InstalledAddonDisplay {
                name: view.name,
                addon_version: view.version,
                online: view.online,
            })
            .collect(),
        None => Vec::new(),
    };

    let (catalog_configured, catalog_entries) = match &state.catalog_client {
        Some(client) => {
            let entries = client
                .list_addons()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|entry| CatalogEntryDisplay {
                    deploy_snippet: format!("docker pull ghcr.io/boernmaster/{}:latest", entry.name),
                    name: entry.name,
                    title: entry.title,
                    description: entry.description,
                    source: entry.source,
                })
                .collect();
            (true, entries)
        }
        None => (false, Vec::new()),
    };

    let lang = state.config.read().await.base.lang.clone();
    let template = AddonsTemplate {
        installed,
        catalog_configured,
        catalog_entries,
        version: state.version.clone(),
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
```

- [ ] **Step 4: Write the template**

`plugins/list.html` is confirmed to be a **fully standalone HTML document** — its own `<!DOCTYPE html>`/`<head>`/`<body>`, no `{% extends %}`/`{% include %}` at all (there is a `templates/base.html` file in this repo, but the Plugins page doesn't use it). The nav bar (`<nav class="navbar">...</nav>`, confirmed at lines 139-202) is duplicated verbatim into every page template rather than shared. `.plugins-grid`/`.plugin-card`/`.empty-state` are defined in this template's own inline `<style>` block, not in the shared `/static/css/style.css` — that shared stylesheet only has generic `.badge`/`.badge-success`/`.badge-danger`/`.alert`/`.table` classes (confirmed in `static/css/style.css`), which is exactly what should be reused for the online/offline indicator instead of inventing new badge variants.

Create `crates/web-ui/templates/addons/list.html` as a standalone document following this exact structure:

```html
<!DOCTYPE html>
<html lang="{{ lang }}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Addons - RustyLox</title>
    <link rel="icon" type="image/svg+xml" href="/static/favicon.svg">
    <link rel="stylesheet" href="/static/css/style.css">
    <script src="/static/js/htmx.min.js"></script>
    <script src="/static/js/i18n.js"></script>
    <style>
        .addons-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 24px;
        }
        .tabs {
            display: flex;
            gap: 8px;
            margin-bottom: 24px;
            border-bottom: 1px solid var(--border-color, #ddd);
        }
        .tab-button {
            padding: 8px 16px;
            border: none;
            background: none;
            cursor: pointer;
            font-size: 1rem;
            border-bottom: 2px solid transparent;
        }
        .tab-button.active {
            border-bottom-color: var(--primary-color, #007bff);
            font-weight: 600;
        }
        .tab-content { display: none; }
        .tab-content.active { display: block; }
        .addons-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
            gap: 16px;
        }
        .addon-card {
            border: 1px solid var(--border-color, #ddd);
            border-radius: 8px;
            padding: 16px;
        }
        .empty-state {
            text-align: center;
            padding: 48px 16px;
            color: var(--text-muted, #666);
        }
    </style>
</head>
<body>
    <nav class="navbar">
        <div class="nav-brand">
            <a href="/" style="display: flex; align-items: center; gap: 8px; text-decoration: none;">
                <img src="/static/logo.svg" alt="RustyLox" style="height: 36px;">
            </a>
        </div>
        <ul class="nav-menu">
            <li><a href="/">Dashboard</a></li>
            <li class="nav-group">
                <a class="nav-group-label" href="/miniserver">Miniserver</a>
                <ul class="nav-dropdown">
                    <li><a href="/miniserver">Overview</a></li>
                    <li><a href="/miniserver/monitor">Comm. Monitor</a></li>
                    <li><a href="/miniserver/backup">MS Backup</a></li>
                </ul>
            </li>
            <li class="nav-group">
                <a class="nav-group-label" href="/mqtt/config">MQTT</a>
                <ul class="nav-dropdown">
                    <li><a href="/mqtt/config">Configuration</a></li>
                    <li><a href="/mqtt/stats">Statistics</a></li>
                </ul>
            </li>
            <li class="nav-group">
                <a class="nav-group-label" href="/weather">Weather</a>
                <ul class="nav-dropdown">
                    <li><a href="/weather">Current &amp; Forecast</a></li>
                    <li><a href="/weather/config">Configuration</a></li>
                </ul>
            </li>
            <li class="nav-group">
                <a class="nav-group-label" href="/plugins">Plugins</a>
                <ul class="nav-dropdown">
                    <li><a href="/plugins">Installed</a></li>
                    <li><a href="/plugins/install">Install Plugin</a></li>
                </ul>
            </li>
            <li class="nav-group">
                <a class="nav-group-label active" href="/addons">Addons</a>
                <ul class="nav-dropdown">
                    <li><a href="/addons">Installed &amp; Catalog</a></li>
                </ul>
            </li>
            <li class="nav-group">
                <a class="nav-group-label" href="/system-health">System</a>
                <ul class="nav-dropdown">
                    <li><a href="/system-health">Health</a></li>
                    <li><a href="/logs">Logs</a></li>
                    <li><a href="/backup">Backup</a></li>
                    <li><a href="/tasks">Tasks</a></li>
                    <li><a href="/network">Network</a></li>
                    <li><a href="/email">Email</a></li>
                    <li><a href="/system-update">Update</a></li>
                    <li><a href="/settings">Settings</a></li>
                </ul>
            </li>
            <li class="nav-group">
                <a class="nav-group-label" href="/admin/users">Admin</a>
                <ul class="nav-dropdown">
                    <li><a href="/admin/users">Users</a></li>
                    <li><a href="/admin/api-keys">API Keys</a></li>
                    <li><a href="/admin/audit">Audit Log</a></li>
                    <li><a href="/admin/security">Security</a></li>
                    <li><a href="/admin/database">Database</a></li>
                    <li><a href="/api-docs">API Docs</a></li>
                </ul>
            </li>
            <li><a href="/profile">Profile</a></li>
        </ul>
    </nav>

    <main class="container">
        <div class="addons-header">
            <h1>Addons</h1>
        </div>

        <div class="tabs">
            <button class="tab-button active" onclick="showAddonsTab('installed', this)">Installed</button>
            <button class="tab-button" onclick="showAddonsTab('catalog', this)">Catalog</button>
        </div>

        <div id="installed" class="tab-content active">
            {% if installed.is_empty() %}
            <div class="empty-state">
                <p>No addons have registered yet. Start a containerized addon with RUSTYLOX_URL set to this instance's address.</p>
            </div>
            {% else %}
            <div class="addons-grid">
                {% for addon in installed %}
                <div class="addon-card">
                    <h3>{{ addon.name }}</h3>
                    <span class="badge {% if addon.online %}badge-success{% else %}badge-danger{% endif %}">
                        {% if addon.online %}online{% else %}offline{% endif %}
                    </span>
                    <p>version {{ addon.addon_version }}</p>
                    <a href="/addons/{{ addon.name }}/settings" class="btn-sm btn-primary">Settings</a>
                </div>
                {% endfor %}
            </div>
            {% endif %}
        </div>

        <div id="catalog" class="tab-content">
            {% if !catalog_configured %}
            <div class="empty-state">
                <p>Catalog not configured - set GITHUB_PACKAGES_TOKEN to enable addon discovery from ghcr.io.</p>
            </div>
            {% else if catalog_entries.is_empty() %}
            <div class="empty-state">
                <p>No addon images found under ghcr.io/boernmaster with the io.rustylox.addon label.</p>
            </div>
            {% else %}
            <div class="addons-grid">
                {% for entry in catalog_entries %}
                <div class="addon-card">
                    <h3>{{ entry.title }}</h3>
                    <p>{{ entry.description }}</p>
                    <a href="{{ entry.source }}">{{ entry.source }}</a>
                    <pre>{{ entry.deploy_snippet }}</pre>
                </div>
                {% endfor %}
            </div>
            {% endif %}
        </div>
    </main>

    <footer class="footer">
        <p>RustyLox {{ version }} | &copy; 2026</p>
    </footer>

    <script>
        function showAddonsTab(id, button) {
            document.querySelectorAll('.tab-content').forEach(el => el.classList.remove('active'));
            document.querySelectorAll('.tab-button').forEach(el => el.classList.remove('active'));
            document.getElementById(id).classList.add('active');
            button.classList.add('active');
        }
    </script>
</body>
</html>
```

- [ ] **Step 5: Register the handler module and route**

Modify `crates/web-ui/src/handlers/mod.rs` — add:

```rust
pub mod addons;
```

Modify `crates/web-ui/src/lib.rs` — add a route inside `create_ui_router`, next to the existing `/plugins` route registration:

```rust
        .route("/addons", get(handlers::addons::list))
```

- [ ] **Step 6: Add a nav link to every other page template**

Confirmed: the nav bar is duplicated verbatim into every page template (no shared layout), so every existing template needs the same "Addons" nav-group added, not just the new one. Find every file with the duplicated nav block:

```bash
cd /home/boern/apps/RustyLox
grep -rl 'nav-group-label" href="/plugins"' crates/web-ui/templates/
```

In each matched file, insert this block immediately after the existing Plugins `<li class="nav-group">...</li>` block (same snippet used in `addons/list.html` above, minus the `active` class since only the Addons page itself should have it):

```html
            <li class="nav-group">
                <a class="nav-group-label" href="/addons">Addons</a>
                <ul class="nav-dropdown">
                    <li><a href="/addons">Installed &amp; Catalog</a></li>
                </ul>
            </li>
```

- [ ] **Step 7: Build**

```bash
cd /home/boern/apps/RustyLox
cargo build --workspace
```

Expected: builds clean (Askama compiles templates at build time, so a template syntax error will fail here)

- [ ] **Step 8: Manual verification**

```bash
cd /home/boern/apps/RustyLox
docker compose up -d --build
curl -s http://localhost:8080/addons | grep -o '<h1>Addons</h1>'
```

Expected: `<h1>Addons</h1>` printed

- [ ] **Step 9: Commit**

```bash
cd /home/boern/apps/RustyLox
git add crates/web-ui
git commit -m "feat(web-ui): add Addons page with Catalog and Installed tabs"
```

---

### Task 11: Capstone end-to-end integration test

**Files:**
- Create: `crates/web-api/tests/addons_end_to_end.rs`

**Interfaces:** none new — exercises Tasks 6/7 together against a fake addon server, proving the full register → discover → edit-settings loop works through the real router, not just unit-level.

- [ ] **Step 1: Write the test**

```rust
use addon_registry::Registry;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use http_body_util::BodyExt;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::util::ServiceExt;
use web_api::{create_router, AppState};

async fn spawn_fake_addon() -> String {
    async fn schema() -> Json<serde_json::Value> {
        Json(json!([{"key": "MQTT_HOST", "label": "MQTT Host", "type": "text", "help": "", "secret": false}]))
    }
    async fn config() -> Json<serde_json::Value> {
        Json(json!({"MQTT_HOST": {"value": "10.0.0.32", "secret_set": false}}))
    }
    async fn save(Json(_body): Json<serde_json::Value>) -> Json<serde_json::Value> {
        Json(json!({"saved": true}))
    }

    let app = Router::new()
        .route("/addon/schema", get(schema))
        .route("/addon/config", get(config).post(save));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

fn test_state(registry: Arc<Registry>) -> AppState {
    let tmp = PathBuf::from("/tmp/rustylox-e2e-test");
    let config_dir = tmp.join("config/system");
    std::fs::create_dir_all(&config_dir).ok();

    let config_manager = rustylox_config::ConfigManager::new(&config_dir);
    let config = rustylox_config::GeneralConfig::default();

    AppState::new(tmp.clone(), "test".to_string(), config_manager, config, None)
        .with_addon_registry(registry, tmp.join("data/system/addonregistry.json"))
}

async fn body_string(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

#[tokio::test]
async fn full_register_discover_configure_loop() {
    let fake_addon_url = spawn_fake_addon().await;
    let registry = Arc::new(Registry::new());
    let app = create_router(test_state(registry));

    // 1. Addon self-registers
    let register_body = json!({
        "name": "kia-connect-bridge",
        "version": "1.0.0",
        "config_api_base_url": fake_addon_url
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/addons/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // 2. RustyLox lists it as online
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/api/addons").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = body_string(response).await;
    let instances: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(instances[0]["name"], "kia-connect-bridge");
    assert_eq!(instances[0]["online"], true);

    // 3. RustyLox fetches its schema through the proxy
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/addons/kia-connect-bridge/schema")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    let schema: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(schema[0]["key"], "MQTT_HOST");

    // 4. RustyLox saves a config change through the proxy
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/addons/kia-connect-bridge/config")
                .header("content-type", "application/json")
                .body(Body::from(json!({"MQTT_HOST": "10.0.0.99"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run it**

```bash
cd /home/boern/apps/RustyLox
cargo test -p web-api full_register_discover_configure_loop
```

Expected: 1 passed

- [ ] **Step 3: Run the entire workspace test suite one final time**

```bash
cd /home/boern/apps/RustyLox
cargo test --workspace
cargo clippy --all --all-targets -- -D warnings
```

Expected: all green, no clippy warnings

- [ ] **Step 4: Commit**

```bash
cd /home/boern/apps/RustyLox
git add crates/web-api/tests/addons_end_to_end.rs
git commit -m "test: add end-to-end register/discover/configure integration test"
```

---

## Manual verification after both tracks are done

1. Build and push a real `kia-connect-bridge` tag (`git tag v1.0.0 && git push origin v1.0.0`), confirm the CI workflow publishes to `ghcr.io/boernmaster/kia-connect-bridge`.
2. Set `GITHUB_PACKAGES_TOKEN`/`GITHUB_PACKAGES_USER` on the live RustyLox instance, restart it, confirm `http://10.0.0.32:8080/addons` Catalog tab shows the real `kia-connect-bridge` entry.
3. Set `RUSTYLOX_URL=http://10.0.0.32:8080` in `kia-connect-bridge`'s `.env`, `docker compose up -d --force-recreate`, confirm it shows up in the Installed tab within 60s and its settings form works end-to-end (edit `KIA_POLL_INTERVAL_SECONDS`, save, verify `.env` updated on disk).
