# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**RustyLox** is a modern Rust rewrite of LoxBerry, an open-source smart home platform that extends Loxone Smart Home systems. It provides MQTT integration, plugin management, Miniserver communication, and a web-based management interface.

**Original Project**: [LoxBerry](https://github.com/mschlenstedt/Loxberry) (Perl/PHP/Bash)
**This Rewrite**: Rust + Docker with backward compatibility for existing plugins

**Key Goals**:
- Modern, type-safe Rust implementation
- Docker-first deployment
- Maintain compatibility with existing LoxBerry plugins
- Async/await architecture for performance
- Server-rendered web UI (no SPA complexity)
- Production-ready features (monitoring, security, HA)

## Technology Stack

### Core Languages
- **Rust 1.80+** (Edition 2021) - Primary language
- **Perl/PHP/Bash** - Plugin compatibility layer (embedded in Docker)

### Web & API
- **Axum 0.7** - Web framework and REST API
- **Askama** - Server-side template engine (type-safe)
- **HTMX** - Progressive enhancement (not SPA)
- **Tower-HTTP** - Middleware (CORS, compression, static files)

### Async & Messaging
- **Tokio** - Async runtime
- **rumqttc 0.24** - MQTT client
- **Broadcast channels** - Real-time event streaming

### Data & Storage
- **serde/serde_json** - Serialization (JSON config files)
- **serde_ini** - INI file parsing (plugin configs)
- **DashMap** - Concurrent hashmaps
- **JSON file-backed stores** - All persistence via atomic temp-file-then-rename writes (no SQL/ORM)

### DevOps
- **Docker** - Multi-stage builds
- **docker-compose** - Stack orchestration
- **GitHub Actions** - CI/CD
- **Prometheus** - Metrics collection & monitoring

## Repository Structure

```
loxberry-rust/
├── crates/                         # Rust workspace crates
│   ├── rustylox-core/             # Common types, errors
│   ├── rustylox-config/           # JSON config management
│   ├── rustylox-logging/          # Logging framework
│   ├── miniserver-client/         # HTTP/UDP Miniserver client
│   ├── mqtt-gateway/              # MQTT gateway with transformers
│   ├── plugin-manager/            # Plugin lifecycle management
│   ├── auth/                      # Authentication & authorization (JWT, RBAC)
│   ├── metrics/                   # System info & metrics collection
│   ├── email-manager/             # Email notifications (SMTP)
│   ├── task-scheduler/            # Scheduled tasks (cron-like)
│   ├── backup-manager/            # Backup/restore functionality
│   ├── web-api/                   # REST API (Axum)
│   ├── web-ui/                    # Server-rendered UI (Askama + HTMX)
│   └── rustylox-daemon/           # Main binary orchestrator
│
├── static/                         # Static assets (CSS, JS, icons)
│   ├── css/style.css
│   ├── js/htmx.min.js
│   ├── favicon.svg
│   └── logo.svg
│
├── volumes/                        # Docker volume mounts (gitignored)
│   ├── config/                    # Configuration files
│   │   └── system/
│   │       ├── general.json
│   │       ├── mqtt_subscriptions.cfg
│   │       └── mqtt_transformers.cfg
│   ├── data/                      # Data storage
│   │   └── system/
│   │       └── plugindatabase.json
│   └── log/                       # Log files
│
├── sdk/                            # SDK libraries for plugin compatibility
│   ├── perllib/                   # Perl SDK modules
│   ├── phplib/                    # PHP SDK libraries
│   └── bashlib/                   # Bash SDK functions
│
├── examples/                       # Example plugins
│   └── sample-plugin/
│
├── Dockerfile                      # Multi-stage Docker build
├── docker-compose.yml              # Service stack definition
├── Cargo.toml                      # Workspace root
├── README.md                       # Project overview
├── ROADMAP.md                      # Development roadmap
├── CONTRIBUTING.md                 # Contribution guidelines
├── CHANGELOG.md                    # Version history
└── docs/archive/                   # Archived phase documentation
```

## Development Environment

### Prerequisites
- **Rust 1.80+** - Install via [rustup](https://rustup.rs/)
- **Docker & Docker Compose** - For containerized development
- **Git** - Version control

### Local Development Setup

```bash
# Clone repository
git clone https://github.com/boernmaster/RustyLox.git
cd RustyLox

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Create volume directories
mkdir -p volumes/config/system volumes/data/system volumes/log/system

# Build all crates
cargo build

# Run tests
cargo test

# Run locally (without Docker)
LBHOMEDIR=/tmp/loxberry cargo run --bin rustylox-daemon

# Or build and run with Docker
docker compose build
docker compose up -d

# View logs
docker compose logs -f rustylox
```

### Editor Setup

**VS Code Extensions**:
- rust-analyzer (official Rust extension)
- Even Better TOML
- Error Lens
- Docker

**IntelliJ IDEA/CLion**:
- Rust plugin
- TOML plugin

## Workspace Structure

This is a **Cargo workspace** with multiple crates. Each crate is independent with its own `Cargo.toml`.

### Crate Dependencies (Bottom-Up)

```
rustylox-daemon (binary)
├── web-ui (templates, handlers)
│   ├── web-api (REST API)
│   │   ├── mqtt-gateway (MQTT client)
│   │   ├── plugin-manager (plugin lifecycle)
│   │   ├── miniserver-client (Miniserver communication)
│   │   ├── metrics (system info)
│   │   └── rustylox-config (config management)
│   │       └── rustylox-core (types, errors)
│   └── rustylox-core
└── rustylox-logging
```

### When to Create a New Crate

Create a new crate when:
- The component has clear boundaries and responsibilities
- It could be reused by other parts of the system
- It has its own set of dependencies
- It represents a major feature area

**Don't create** a crate for:
- Small utility functions (add to rustylox-core)
- Single-use helpers
- Tightly coupled code

## Common Development Tasks

### Adding a New REST API Endpoint

1. **Define route in `web-api/src/lib.rs`**:
```rust
Router::new()
    .route("/api/new-feature", get(handlers::new_feature::list))
    .route("/api/new-feature/:id", post(handlers::new_feature::update))
```

2. **Create handler in `web-api/src/routes/new_feature.rs`**:
```rust
use axum::{extract::State, Json};
use crate::AppState;

pub async fn list(State(state): State<AppState>) -> Json<Vec<Item>> {
    // Implementation
}
```

3. **Add to module in `web-api/src/routes/mod.rs`**:
```rust
pub mod new_feature;
```

4. **Write tests in `web-api/tests/new_feature_test.rs`**:
```rust
#[tokio::test]
async fn test_list_endpoint() {
    // Test implementation
}
```

### Adding a New Web UI Page

1. **Create template in `web-ui/templates/feature.html`**:
```html
<!DOCTYPE html>
<html>
<head>
    <link rel="icon" type="image/svg+xml" href="/static/favicon.svg">
    <link rel="stylesheet" href="/static/css/style.css">
</head>
<body>
    <!-- Your template -->
</body>
</html>
```

2. **Define template struct in `web-ui/src/templates.rs`**:
```rust
#[derive(Template)]
#[template(path = "feature.html")]
pub struct FeatureTemplate {
    pub items: Vec<Item>,
}
```

3. **Create handler in `web-ui/src/handlers/feature.rs`**:
```rust
pub async fn show(State(state): State<AppState>) -> Html<String> {
    let template = FeatureTemplate { items: vec![] };
    Html(template.render().unwrap_or_default())
}
```

4. **Add route in `web-ui/src/lib.rs`**:
```rust
Router::new()
    .route("/feature", get(handlers::feature::show))
```

### Modifying Configuration Structure

1. **Update struct in `rustylox-config/src/general.rs`**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSection {
    pub field: String,
}
```

2. **Add to GeneralConfig**:
```rust
pub struct GeneralConfig {
    // ...existing fields
    pub new_section: NewSection,
}
```

3. **Update default implementation**:
```rust
impl Default for NewSection {
    fn default() -> Self {
        Self { field: String::new() }
    }
}
```

4. **Migration**: If changing existing config, add migration logic

### Adding a Plugin Lifecycle Hook

Plugin hooks are shell scripts executed at specific points:

**Hook Types**:
- `preroot.sh` - Runs as root before installation
- `preinstall.sh` - Runs as loxberry user before installation
- `preupgrade.sh` - Runs as loxberry user before upgrade
- `postinstall.sh` - Runs after installation
- `postupgrade.sh` - Runs after upgrade
- `postroot.sh` - Runs as root after installation/upgrade
- `uninstall.sh` - Runs during uninstallation

**Hook Execution** (`plugin-manager/src/installer.rs`):
```rust
async fn execute_hook(&self, hook_path: &Path, plugin: &PluginEntry) -> Result<()> {
    let env = self.build_plugin_env(plugin);

    tokio::process::Command::new("bash")
        .arg(hook_path)
        .envs(env)
        .output()
        .await?;

    Ok(())
}
```

## Code Style & Conventions

### Rust Conventions

**Always follow**:
- `cargo fmt` before committing (enforced in CI)
- `cargo clippy` and fix warnings
- Snake_case for functions and variables
- PascalCase for types and traits
- SCREAMING_SNAKE_CASE for constants

**Error Handling**:
```rust
// ✅ Good - use Result and ?
pub async fn do_something() -> Result<Value> {
    let data = fetch_data().await?;
    Ok(data)
}

// ❌ Bad - don't panic in library code
pub fn do_something() -> Value {
    fetch_data().unwrap()  // Don't do this!
}
```

**Async Functions**:
```rust
// ✅ Good - async for I/O operations
pub async fn read_config() -> Result<Config> {
    tokio::fs::read_to_string("config.json").await?
    // ...
}

// ✅ Good - sync for pure computation
pub fn calculate_hash(data: &[u8]) -> String {
    // No I/O, no async needed
}
```

### Naming Conventions

**Files**:
- `snake_case.rs` for modules
- `lib.rs` for crate root
- `main.rs` for binary crates

**Crates**:
- `kebab-case` in Cargo.toml
- `snake_case` in code (hyphens become underscores)

**Examples**:
```toml
[package]
name = "rustylox-config"  # Kebab-case in Cargo.toml

# In code:
use rustylox_config::Config;  // Snake_case
```

### Documentation

**Add documentation for**:
- All public APIs
- Complex algorithms
- Non-obvious behavior

```rust
/// Sends a command to the Miniserver via HTTP.
///
/// # Arguments
/// * `params` - Key-value pairs to send
///
/// # Returns
/// HashMap of parameter names to success status
///
/// # Example
/// ```
/// let result = client.send(vec![("V1", "100")]).await?;
/// ```
pub async fn send(&self, params: Vec<(String, String)>) -> Result<HashMap<String, bool>>
```

**Don't document**:
- Obvious getters/setters
- Private implementation details
- Test functions

## Testing Strategy

### Unit Tests

Place in same file as code:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let config = parse("{\"key\":\"value\"}").unwrap();
        assert_eq!(config.key, "value");
    }
}
```

### Integration Tests

Place in `tests/` directory:
```rust
// crates/web-api/tests/integration_test.rs
use web_api::create_router;

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_router();
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
}
```

### Testing Docker Build

```bash
# Build
docker compose build

# Test services start
docker compose up -d

# Check logs
docker compose logs

# Test API
curl http://localhost/health

# Cleanup
docker compose down
```

## Important Rules

### DO

✅ **Read existing code** before making changes
✅ **Follow the established patterns** in similar files
✅ **Write tests** for new functionality
✅ **Update documentation** when changing public APIs
✅ **Use type-safe patterns** (avoid `unsafe`, minimize `unwrap()`)
✅ **Handle errors properly** with `Result<T, Error>`
✅ **Use async/await** for I/O operations
✅ **Keep functions focused** (single responsibility)
✅ **Reuse existing types** from rustylox-core where possible

### DON'T

❌ **Don't add dependencies lightly** - discuss first if adding a heavy crate
❌ **Don't use `unwrap()` or `expect()`** in production code (use `?` instead)
❌ **Don't block async runtime** with sync operations
❌ **Don't create new JSON config formats** without discussion
❌ **Don't break backward compatibility** with existing LoxBerry configs
❌ **Don't add emojis to code** unless explicitly requested
❌ **Don't create files unless necessary** - prefer editing existing files
❌ **Don't over-engineer** - KISS principle applies

### Async/Await Rules

```rust
// ✅ Good - async I/O
pub async fn read_file(path: &Path) -> Result<String> {
    tokio::fs::read_to_string(path).await
}

// ❌ Bad - blocking in async context
pub async fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)  // Blocks the async runtime!
}

// ✅ Good - parallel async operations
let (config, plugins) = tokio::join!(
    load_config(),
    load_plugins()
);

// ❌ Bad - sequential when could be parallel
let config = load_config().await;
let plugins = load_plugins().await;
```

## Configuration Files

### JSON Configuration (`config/system/general.json`)

**Structure**:
```json
{
  "Base": {
    "Version": "4.0.0.0",
    "Lang": "en",
    "Systemloglevel": "6"
  },
  "Miniserver": {
    "1": {
      "Name": "MS1",
      "Ipaddress": "192.168.1.100",
      "Port": "80",
      "Admin": "admin",
      "Pass": "password"
    }
  },
  "Mqtt": {
    "Brokerhost": "mosquitto",
    "Brokerport": "1883",
    "Udpinport": "11884"
  }
}
```

**Accessing**:
```rust
let config = state.config.read().await;
let miniserver = config.miniserver.get("1")?;
```

### INI Configuration (`config/system/mqtt_subscriptions.cfg`)

**Structure**:
```ini
[HomeTemperature]
TOPIC=home/+/temperature
NAME=Temperature Sensors
FILTER=_healthcheck_|_info_
ENABLED=1
```

**Parsing**:
```rust
fn parse_subscriptions_cfg(content: &str) -> Vec<Subscription> {
    // Parse with serde_ini or manual parsing
}
```

## Plugin Development

### Plugin Structure

```
plugin-name/
├── plugin.cfg              # Required - plugin metadata
├── preroot.sh             # Optional - runs as root before install
├── preinstall.sh          # Optional - runs as loxberry before install
├── preupgrade.sh          # Optional - runs as loxberry before upgrade
├── postinstall.sh         # Optional - runs after install
├── postupgrade.sh         # Optional - runs after upgrade
├── postroot.sh            # Optional - runs as root after install/upgrade
├── uninstall.sh           # Optional - runs during uninstall
├── daemon/                # Optional - background daemons
│   └── daemon.pl
└── webfrontend/           # Optional - web interface
    └── htmlauth/
        └── index.html
```

### plugin.cfg Format

```ini
[AUTHOR]
NAME=John Doe
EMAIL=john@example.com

[PLUGIN]
NAME=myplugin
FOLDER=myplugin
VERSION=1.0.0
TITLE_EN=My Plugin
TITLE_DE=Mein Plugin

[SYSTEM]
REBOOT=false
ARCHITECTURE=all
```

### Environment Variables for Plugins

When executing plugin hooks, these are automatically injected:

```bash
LBHOMEDIR=/opt/loxberry
LBPPLUGINDIR=myplugin
LBPHTMLDIR=/opt/loxberry/webfrontend/html/plugins/myplugin
LBPHTMLAUTHDIR=/opt/loxberry/webfrontend/htmlauth/plugins/myplugin
LBPDATADIR=/opt/loxberry/data/plugins/myplugin
LBPLOGDIR=/opt/loxberry/log/plugins/myplugin
LBPCONFIGDIR=/opt/loxberry/config/plugins/myplugin
```

## Git Workflow

### Branch Strategy

**Main Branch**: `main` - Production-ready code
**Development**: Feature branches from `main`

### Creating a Feature Branch

```bash
git checkout main
git pull origin main
git checkout -b feature/your-feature-name
```

### Commit Message Format

Use conventional commits:

```
type(scope): brief description

Detailed explanation if needed

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

**Types**:
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation only
- `style` - Code style (formatting, no logic change)
- `refactor` - Code refactoring
- `test` - Adding tests
- `chore` - Build/tooling changes

**Examples**:
```
feat(mqtt): add RegEx filter support for subscriptions
fix(plugin): handle nested ZIP archives correctly
docs(readme): update installation instructions
refactor(config): simplify JSON parsing logic
```

### Before Committing

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Run tests
cargo test

# Build Docker image
docker compose build
```

### Before Tagging a Release

**ALWAYS write release notes before creating a version tag.**

1. Update `CHANGELOG.md` with a summary of changes grouped by type (`feat`, `fix`, `perf`, etc.)
2. Commit the changelog update
3. Create and push the tag — use the same release notes as the tag annotation message
4. The GitHub Release (created by `release.yml`) must also contain the release notes — update the `body:` field in `release.yml` or use `gh release edit` after the fact

### Pull Request Process

1. Create feature branch
2. Make changes and commit
3. Push to GitHub: `git push origin feature/your-feature-name`
4. Open Pull Request to `main`
5. Wait for CI checks to pass
6. Address review feedback
7. Squash commits if needed
8. Merge when approved

## CI/CD Pipeline

**GitHub Actions** (`.github/workflows/`)

### Workflows

**test.yml** - Continuous Integration:
- Runs on every push and PR
- `cargo fmt --check`
- `cargo clippy`
- `cargo test --all`

**security-scan.yml** - Security Scanning:
- `cargo audit` via rustsec (continue-on-error, non-blocking)

**docker-publish.yml** - Docker Builds:
- Triggered on tags (`v*.*.*`)
- Multi-platform builds (amd64, arm64 via QEMU)
- Pushes to `ghcr.io/boernmaster/rustylox`

**release.yml** - GitHub Release:
- Triggered on tags (`v*.*.*`)
- Builds x86_64 binary
- Creates GitHub Release with release notes

## Docker Development

### Multi-Stage Dockerfile

**Stages**: chef (cargo-chef planner) → builder (release compile) → runtime

**Builder** uses prebuilt `lukemathwalker/cargo-chef` image and builds `rustylox-daemon`.

**Runtime**:
```dockerfile
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y \
    perl php7.4-cli bash ca-certificates
COPY --from=builder /build/target/release/rustylox-daemon /usr/local/bin/
COPY static /opt/loxberry/static
COPY sdk /opt/loxberry/sdk
```

### docker-compose.yml

```yaml
services:
  rustylox:
    build: .
    ports:
      - "80:80"           # Web UI & API
      - "6066:6066/udp"   # Miniserver UDP in
      - "8090:8090/udp"   # Miniserver UDP out
      - "11884:11884/udp" # MQTT UDP
    volumes:
      - ./volumes/config:/opt/loxberry/config
      - ./volumes/data:/opt/loxberry/data
      - ./volumes/log:/opt/loxberry/log
    environment:
      - RUST_LOG=debug

  mosquitto:
    image: eclipse-mosquitto:2.0
    ports:
      - "1883:1883"
      - "9001:9001"
```

### Common Docker Commands

```bash
# Build
docker compose build

# Start services
docker compose up -d

# View logs
docker compose logs -f rustylox

# Stop services
docker compose down

# Rebuild and restart
docker compose down && docker compose build && docker compose up -d

# Execute command in container
docker compose exec rustylox bash

# View container stats
docker stats rustylox
```

## Debugging

### Logging Levels

**Set via environment variable**:
```bash
# All debug output
RUST_LOG=debug cargo run

# Specific crate
RUST_LOG=mqtt_gateway=trace cargo run

# Multiple levels
RUST_LOG=web_api=debug,mqtt_gateway=trace cargo run
```

**In Code**:
```rust
use tracing::{debug, info, warn, error};

debug!("Detailed debug info: {:?}", value);
info!("Normal information");
warn!("Warning message");
error!("Error occurred: {}", err);
```

### VS Code Launch Configuration

`.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug rustylox-daemon",
      "cargo": {
        "args": ["build", "--bin=rustylox-daemon"]
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug",
        "LBHOMEDIR": "/tmp/loxberry"
      }
    }
  ]
}
```

### Common Issues

**Problem**: "Permission denied" when creating plugin directories
**Solution**: Run with proper permissions or in Docker

**Problem**: MQTT messages not appearing in monitor
**Solution**: Check MQTT broker is running, check subscriptions config

**Problem**: Template compilation errors
**Solution**: Ensure template struct fields match template variables

**Problem**: Async runtime blocking
**Solution**: Don't use `std::fs` or other blocking I/O in async functions

## Project Status

**Current Status**: Production-ready (v0.8.0). All core features are implemented — MQTT gateway, plugin system, web UI (31 Askama templates), security hardening (JWT/RBAC, API keys, audit log), monitoring, backup/restore, email, task scheduling, system updates, and admin panel.

**MQTT UI**: Incoming Overview and MQTT Finder are tabs on `/mqtt/config` (not separate pages) since v0.8.0.

**Storage**: All persistence is JSON file-backed (`auth.json`, `plugindatabase.json`, `task_history.json`, etc.) — no SQL database.

Next planned work: advanced features & ecosystem expansion (plugin marketplace, Kubernetes, OAuth2/OIDC, PWA). See [ROADMAP.md](ROADMAP.md) for details.

Historical phase documentation is archived in [docs/archive/](docs/archive/).

## Additional Resources

### Documentation
- [README.md](README.md) - Project overview
- [ROADMAP.md](ROADMAP.md) - Development roadmap
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
- [CHANGELOG.md](CHANGELOG.md) - Version history

### External Links
- **Rust Book**: https://doc.rust-lang.org/book/
- **Tokio Guide**: https://tokio.rs/tokio/tutorial
- **Axum Docs**: https://docs.rs/axum/latest/axum/
- **Askama Guide**: https://djc.github.io/askama/
- **HTMX Docs**: https://htmx.org/docs/

### Original LoxBerry
- **Wiki**: https://wiki.loxberry.de/
- **Forum**: https://www.loxforum.com/forum/german/software-konfiguration-und-programmierung/loxberry
- **Original Repo**: https://github.com/mschlenstedt/Loxberry

## Questions?

If you're unsure about:
- Architecture decisions - Check [ROADMAP.md](ROADMAP.md)
- How to contribute - See [CONTRIBUTING.md](CONTRIBUTING.md)
- Specific features - Look at existing similar code or check [docs/archive/](docs/archive/)
- Code patterns - Look at existing similar code first

When in doubt:
1. Read existing code for patterns
2. Check documentation
3. Ask in GitHub Discussions
4. Open an issue for clarification
