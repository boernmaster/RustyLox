# Changelog

All notable changes to RustyLox will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.22] - 2026-03-22

### Fixed
- MQTT monitor and Miniserver monitor no longer show a blank message list after the DOM refactor in v0.6.21
  - `wrapper.firstChild` was returning a whitespace text node (leading newline from the template literal) instead of the actual `<div>` element
  - Fixed by using `wrapper.firstElementChild` which correctly skips text nodes

## [0.6.21] - 2026-03-22

### Added
- Documentation for Vitoconnect plugin integration (`docs/vitoconnect-integration.md`)
  - Data flow from Viessmann API to Miniserver (MQTT and HTTP)
  - Sending commands from Miniserver to Viessmann via HTTP Virtual Outputs
  - Complete reference of all supported `setvalue` parameters
  - Plugin configuration, troubleshooting, and prerequisites

## [0.6.20] - 2026-03-22

### Added
- Authentication middleware now enforces login redirect for the entire web UI, not just individual routes
- Unauthenticated requests are redirected to `/login?redirect=<original_path>` so users return to their destination after login
- Session cookie (`lb_token`) is set on successful login and cleared on logout
- Public paths (`/login`, `/static/`, `/health`) bypass auth checks

## [0.6.19] - 2026-03-22

### Fixed
- Backup ZIP operations now correctly convert `ZipError` to `rustylox_core::Error`
- Fixed Docker build failure caused by missing `From<ZipError>` trait implementation

## [0.6.18] - 2026-03-22

### Changed
- **Breaking**: Renamed all `loxberry-*` crates to `rustylox-*` (`rustylox-core`, `rustylox-config`, `rustylox-logging`, `rustylox-daemon`, `rustylox-metrics`)
- **Breaking**: Prometheus metrics prefix changed from `loxberry_` to `rustylox_`
- Backup format switched from tar.gz to ZIP (Deflate compression)
- Dockerfile and CI binary name updated to `rustylox-daemon`

### Fixed
- Backup metadata now uses the actual build version instead of hardcoded "4.0.0.0"

## [0.6.17] - 2026-03-22

### Added
- Miniserver backup: real-time progress bar during backup download via Server-Sent Events (SSE)
- Backup Now button returns immediately; background task streams per-file progress to the browser
- Progress bar shows file count (done / total), current file path, and animated fill
- SSE endpoint: `GET /miniserver/backup/:id/progress/:job_id`
- JS `EventSource` handles `start`, `file`, `done`, and `backup_error` events

## [0.6.16] - 2026-03-22

### Changed
- Miniserver backup now performs a **full recursive backup** of all data directories: `log`, `prog`, `sys`, `stats`, `temp`, `update`, `web`, `user`
- Filesystem is walked via BFS using `/dev/fslist/<dir>/` — subdirectories are followed automatically
- `/sys/internal/` is skipped (causes errors on many Miniserver firmware versions)
- All files are packed into a single ZIP with directory structure preserved (`prog/Default.Loxone`, `log/2024-01-01.log`, etc.)

## [0.6.15] - 2026-03-22

### Fixed
- CI: clippy errors in `miniserver_backup.rs` — `&PathBuf` → `&Path` (ptr_arg), `std::io::Error::other()` (io_other_error)
- CI: security audit — updated `rustls-webpki` 0.103.9 → 0.103.10 (patches RUSTSEC-2026-0049); 0.102.x series (via rumqttc) acknowledged in `audit.toml`

## [0.6.14] - 2026-03-22

### Changed
- Miniserver backup now downloads **all files** from the `/prog/` directory (`.Loxone` project files, `permissions.bin`, and any other binaries) and packages them into a single `.zip` archive instead of saving only the first `.loxone` file
- Backup filenames now use `.zip` extension (`Backup_<name>_<timestamp>.zip`)
- Log messages report the number of files packed and the total archive size

## [0.6.13] - 2026-03-22

### Added
- Miniserver backup: full backup feature downloading `.loxone` project files via Miniserver filesystem API (`/dev/fslist/prog/`, `/dev/fsget/prog/`)
- Miniserver backup: automatic scheduling per Miniserver (configurable interval: 6h / 12h / 24h / 48h / weekly) with background scheduler
- Miniserver backup: dedicated log file at `log/system/miniserver-backup.log` visible in the log viewer
- Miniserver backup: MS Backup page moved from System menu to Miniserver menu

### Fixed
- Miniserver backup: case-insensitive `.Loxone` extension matching (Miniserver stores files as `.Loxone` not `.loxone`)
- Miniserver backup: fallback search across multiple filesystem paths (`/prog/`, root, `/sd/`) with diagnostic output on failure
- Miniserver backup: correct `Content-Type: application/octet-stream` for file downloads
- CI: consolidated two redundant workflows into one; Docker image now built and pushed only once per tag

## [0.6.12] - 2026-03-22

### Added
- LoxBerry-compatible `/admin/system/tools/logfile.cgi` route for plugin log viewing
- Plugins calling `?logfile=/plugins/Foo/bar.log&header=html&format=template` now render via the built-in log viewer with path traversal protection

## [0.6.11] - 2026-03-22

### Added
- Log LAN IP address at startup so network address is discoverable
- Bind web server port from config (default 80)

### Fixed
- PHP 8 compatibility and plugin web CWD for Vitoconnect
- Web UI fetch error handling and API response mismatches
- Port reverted to 8080 after incorrect config-based change

### Performance
- CI: remove redundant `cargo build` step after `cargo clippy` (clippy already compiles)
- Dockerfile: switch to prebuilt `lukemathwalker/cargo-chef` image (~2-3 min saved per build)

### Chore
- Multiple clippy warning fixes across crates (useless_format, redundant_closure,
  io_other_error, dead_code, unused_imports, ptr_arg, for_kv_map, if_same_then_else,
  manual_range_contains, unused_variables)
- Re-enabled arm64 Docker image builds

## [1.3.0] - 2026-03-17

### Added - Phase 6: Performance & Monitoring
- Database abstraction layer with PostgreSQL and SQLite support
- Email notification system with SMTP and HTML templates
- Task scheduler with cron expression support
- Network diagnostics tools (ping, port scan, connectivity tests)
- System health monitoring (CPU, memory, disk usage)
- Backup and restore functionality
- Enhanced health check endpoint with detailed metrics

### Added - Phase 7: Security Hardening
- JWT authentication with HS256 signing
- Role-Based Access Control (RBAC) with Admin, Operator, Viewer, PluginManager roles
- API key management with `lbx_` prefix and SHA-256 hashing
- Argon2id password hashing for secure credential storage
- Account lockout after 5 failed login attempts
- Security headers middleware (CSP, X-Frame-Options, etc.)
- Comprehensive audit logging for all security-sensitive operations
- Auth REST API endpoints (`/api/auth/*`, `/api/users/*`)
- In-memory session management with automatic expiry
- Default admin user creation on first run

### Fixed
- Disabled rate limiting to resolve Docker API failures
- Fixed "Unable To Extract Key!" errors on all endpoints

### Security
- All passwords now hashed with Argon2id
- JWT tokens for stateless authentication
- API keys stored as SHA-256 hashes
- Security headers on all HTTP responses
- Audit log for compliance and security monitoring

## [1.2.0] - 2026-03-16

### Added - Phase 5: Logging & SDK
- Structured logging framework
- Plugin logging integration
- Configuration validation
- Initial backup/restore functionality

### Changed
- Brand new icon and logo design for RustyLox
- Comprehensive badges in README.md (license, language, CI status, etc.)
- MQTT configuration interface with subscriptions and conversions management
- RegEx filter support for MQTT subscriptions
- Real-time incoming messages viewer in MQTT config
- JSON expansion with boolean conversion display
- Updated all template files to include favicon and logo
- Enhanced README with better structure and badges
- Updated all phase documentation with status badges

### Added
- LICENSE file (Apache 2.0)
- CONTRIBUTING.md with contribution guidelines
- This CHANGELOG.md file

## [1.0.0] - 2026-03-15

### Added
- Complete Rust rewrite of LoxBerry
- Docker containerization with multi-stage builds
- Miniserver HTTP/HTTPS/UDP client
- MQTT Gateway with transformers
- Plugin management system
- Web UI with Askama templates + HTMX
- REST API with Axum
- Server-Sent Events for real-time MQTT monitoring
- Plugin lifecycle hooks
- Configuration management (JSON)
- Delta-sending optimization for Miniserver
- Reboot detection
- CloudDNS support

### Phase Breakdown

#### Phase 1 - Foundation
- Core types and error handling
- Configuration management
- Miniserver client
- REST API foundation
- Docker setup

#### Phase 2 - Plugin System
- Plugin database
- ZIP extraction
- Lifecycle hooks
- Plugin API endpoints
- SDK compatibility prep

#### Phase 3 - MQTT Gateway
- MQTT broker integration
- UDP listener
- Message transformers
- Bidirectional relay
- Hot-reload capability

#### Phase 4 - Web UI
- Dashboard
- Miniserver management
- MQTT monitor
- Plugin management
- Settings page
- Authentication support

## [0.1.0] - 2026-01-15

### Added
- Initial project scaffolding
- Cargo workspace setup
- Basic crate structure

---

## Release Notes

### Version 1.0.0 Highlights

This is the first stable release of RustyLox, a complete rewrite of LoxBerry in Rust. Key features:

**Performance**
- Native compiled code (vs interpreted Perl/PHP)
- Async I/O with Tokio runtime
- Thread-safe concurrent operations
- ~150MB Docker image

**Modern Stack**
- Rust 1.80+ (Edition 2021)
- Axum 0.7 web framework
- Askama templates + HTMX
- rumqttc MQTT client
- Docker containerization

**Compatibility**
- Compatible with LoxBerry configuration format
- MQTT protocol compatibility
- Plugin system (via SDK layer)
- Miniserver communication protocol

**Security**
- Type-safe Rust code
- Memory safety guarantees
- Secure credential storage
- HTTPS support

---

[Unreleased]: https://github.com/boernmaster/RustyLox/compare/v1.3.0...HEAD
[1.3.0]: https://github.com/boernmaster/RustyLox/releases/tag/v1.3.0
[1.2.0]: https://github.com/boernmaster/RustyLox/releases/tag/v1.2.0
[1.0.0]: https://github.com/boernmaster/RustyLox/releases/tag/v1.0.0
[0.1.0]: https://github.com/boernmaster/RustyLox/releases/tag/v0.1.0
