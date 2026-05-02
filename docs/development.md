# Development Guide

## Prerequisites

| Tool | Minimum version | Install |
|------|----------------|---------|
| Rust + Cargo | 1.80 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Docker | 24+ | [docs.docker.com](https://docs.docker.com/get-docker/) |
| Docker Compose | 2.20+ | Bundled with Docker Desktop |
| Git | 2.x | system package manager |

---

## Building

### Docker (recommended)

```bash
git clone https://github.com/boernmaster/RustyLox.git
cd RustyLox
mkdir -p volumes/config/system volumes/data/system volumes/log/system
docker compose build
docker compose up -d
docker compose logs -f rustylox
```

### Native (Cargo)

```bash
# Clone
git clone https://github.com/boernmaster/RustyLox.git
cd RustyLox

# Create local directories
mkdir -p /tmp/loxberry/{config/system,data/system,log/system,static}
cp -r static /tmp/loxberry/

# Build release binary
cargo build --release
# Binary: target/release/rustylox-daemon

# Run
LBHOMEDIR=/tmp/loxberry RUST_LOG=info \
  cargo run --release --bin rustylox-daemon
```

Open **http://localhost:80** (or whichever port `BIND_ADDR` is set to).

---

## Testing

```bash
# Run all tests
cargo test --all

# Run tests for a specific crate
cargo test -p rustylox-core
cargo test -p mqtt-gateway

# Run with stdout output
cargo test -- --nocapture
```

### Integration test against a running stack

```bash
# Health check
curl http://localhost/health

# MQTT publish (requires mosquitto-clients)
docker exec mosquitto mosquitto_pub -t "home/test" -m "hello"

# MQTT UDP gateway
echo '{"topic":"home/sensor","value":"25.5"}' | nc -u localhost 11884
```

---

## Code Quality

```bash
# Format (enforced in CI)
cargo fmt

# Lint (enforced in CI)
cargo clippy

# Security audit
cargo audit
```

CI runs `cargo fmt --check`, `cargo clippy`, `cargo test --all`, and `cargo audit` on every push to `main` and on pull requests.

---

## Editor Setup

**VS Code** — install these extensions:
- `rust-analyzer` (official)
- `Even Better TOML`
- `Error Lens`

**IntelliJ / CLion** — install the Rust plugin.

### VS Code launch config

`.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug rustylox-daemon",
      "cargo": { "args": ["build", "--bin=rustylox-daemon"] },
      "env": {
        "RUST_LOG": "debug",
        "LBHOMEDIR": "/tmp/loxberry"
      }
    }
  ]
}
```

---

## Watch Mode

```bash
cargo install cargo-watch

# Rebuild on save
cargo watch -x build

# Rebuild and run on save
cargo watch -x 'run --bin rustylox-daemon'

# Re-test on save
cargo watch -x test
```

---

## Debugging

```bash
# Verbose logs
RUST_LOG=debug cargo run --bin rustylox-daemon

# Crate-level log scoping
RUST_LOG=mqtt_gateway=trace,web_api=debug cargo run --bin rustylox-daemon

# GDB
rust-gdb ./target/debug/rustylox-daemon

# LLDB
rust-lldb ./target/debug/rustylox-daemon
```

---

## Adding a REST API Endpoint

1. **Define the route** in `crates/web-api/src/lib.rs`:
   ```rust
   Router::new()
       .route("/api/feature", get(handlers::feature::list))
       .route("/api/feature/:id", post(handlers::feature::update))
   ```

2. **Create the handler** in `crates/web-api/src/routes/feature.rs`:
   ```rust
   use axum::{extract::State, Json};
   use crate::AppState;

   pub async fn list(State(state): State<AppState>) -> Json<Vec<Item>> {
       // implementation
   }
   ```

3. **Expose the module** in `crates/web-api/src/routes/mod.rs`:
   ```rust
   pub mod feature;
   ```

4. **Write tests** in `crates/web-api/tests/feature_test.rs`.

---

## Adding a Web UI Page

1. **Create the template** at `crates/web-ui/templates/feature.html`:
   ```html
   {% extends "base.html" %}
   {% block content %}
   <!-- page content -->
   {% endblock %}
   ```

2. **Define the template struct** in `crates/web-ui/src/templates.rs`:
   ```rust
   #[derive(Template)]
   #[template(path = "feature.html")]
   pub struct FeatureTemplate {
       pub items: Vec<Item>,
   }
   ```

3. **Add the handler** in `crates/web-ui/src/handlers/feature.rs`:
   ```rust
   pub async fn show(State(state): State<AppState>) -> Html<String> {
       let template = FeatureTemplate { items: vec![] };
       Html(template.render().unwrap_or_default())
   }
   ```

4. **Register the route** in `crates/web-ui/src/lib.rs`.

---

## Adding a New Crate

Create a new crate when the component has clear boundaries, its own dependencies, and could be reused. For small utilities, add to `rustylox-core` instead.

```bash
cargo new crates/my-feature --lib
```

Then add it to the workspace `Cargo.toml`:
```toml
[workspace]
members = [
    # ...existing crates...
    "crates/my-feature",
]
```

---

## Hot-Reload

While the daemon is running, configuration can be reloaded without a restart:

```bash
# Reload MQTT subscriptions
curl -X POST http://localhost/api/mqtt/subscriptions/reload

# Reload MQTT transformers
curl -X POST http://localhost/api/mqtt/transformers/reload
```

`general.json` changes require a daemon restart.

---

## Troubleshooting Builds

**Port already in use**
```bash
sudo lsof -i :80
# Change BIND_ADDR in docker-compose.yml if needed
```

**Rust version too old**
```bash
rustup update stable
```

**Linking errors (Linux)**
```bash
sudo apt-get install build-essential pkg-config libssl-dev
```

**Out of memory during compile**
```bash
cargo build -j 2   # limit parallel codegen units
```

**Docker permission denied on Cargo.lock**
```bash
sudo chown -R $USER:$USER .
```
