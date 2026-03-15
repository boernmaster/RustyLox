# Changelog

All notable changes to RustyLox will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Brand new icon and logo design for RustyLox
- Comprehensive badges in README.md (license, language, CI status, etc.)
- MQTT configuration interface with subscriptions and conversions management
- RegEx filter support for MQTT subscriptions
- Real-time incoming messages viewer in MQTT config
- JSON expansion with boolean conversion display
- LICENSE file (Apache 2.0)
- CONTRIBUTING.md with contribution guidelines
- This CHANGELOG.md file

### Changed
- Updated all template files to include favicon and logo
- Enhanced README with better structure and badges
- Updated all phase documentation with status badges

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

[Unreleased]: https://github.com/boernmaster/RustyLox/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/boernmaster/RustyLox/releases/tag/v1.0.0
[0.1.0]: https://github.com/boernmaster/RustyLox/releases/tag/v0.1.0
