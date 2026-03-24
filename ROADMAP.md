# RustyLox Development Roadmap

<div align="center">

![Project Status](https://img.shields.io/badge/Project-Active-brightgreen)
![Current Phase](https://img.shields.io/badge/Current%20Phase-7a%20(Complete)-brightgreen)
![Completion](https://img.shields.io/badge/Completion-95%25-brightgreen)

</div>

## Vision

Transform LoxBerry into a modern, secure, and scalable smart home platform built with Rust, while maintaining backward compatibility with the existing plugin ecosystem.

## Project Timeline

```
2024 Q1  ████████████████████  Phase 1: Foundation ✅
2024 Q2  ████████████████████  Phase 2: Plugin System ✅
2024 Q3  ████████████████████  Phase 3: MQTT Gateway ✅
2024 Q4  ████████████████████  Phase 4: Web UI ✅
2025 Q1  ████████████████████  Phase 5: SDK & Logging ✅
2025 Q2  ████████████████████  Phase 6: Performance & Monitoring ✅
2025 Q3  ████████████████████  Phase 7: Security Hardening ✅
2026 Q1  ████████████████████  Phase 7a: Web UI for Backend Features ✅
2026 Q2  ░░░░░░░░░░░░░░░░░░░  Phase 8: Advanced Features & Ecosystem
```

---

## ✅ Phase 1: Foundation (COMPLETE)

**Duration**: 2 months (Jan-Feb 2024)
**Status**: ✅ Complete
**Completion**: 100%

### Objectives
Build the core infrastructure and foundational components.

### Deliverables
- ✅ Cargo workspace with modular crate structure
- ✅ Core types and error handling
- ✅ JSON configuration management
- ✅ Miniserver HTTP/HTTPS/UDP client
- ✅ Delta-sending optimization
- ✅ Miniserver reboot detection
- ✅ REST API with Axum
- ✅ Docker multi-stage build
- ✅ docker-compose setup

### Key Metrics
- **Lines of Code**: 2,500+
- **Crates**: 5
- **API Endpoints**: 10
- **Test Coverage**: Basic unit tests

**[📄 Full Details](PHASE1_COMPLETE.md)**

---

## ✅ Phase 2: Plugin System (COMPLETE)

**Duration**: 2 months (Mar-Apr 2024)
**Status**: ✅ Complete
**Completion**: 100%

### Objectives
Implement complete plugin lifecycle management with backward compatibility.

### Deliverables
- ✅ Plugin database (JSON-based)
- ✅ ZIP extraction and validation
- ✅ plugin.cfg parser (INI format)
- ✅ MD5-based plugin identity
- ✅ Lifecycle hooks (preroot, preinstall, postinstall, postroot, uninstall)
- ✅ Directory isolation per plugin
- ✅ Environment variable injection
- ✅ Plugin API endpoints (install, list, uninstall)
- ✅ Real-world plugin testing (Vitoconnect)

### Key Metrics
- **Lines of Code**: 2,000+
- **Plugins Tested**: 3 (sample, testplugin, Vitoconnect)
- **API Endpoints**: +8
- **Hooks Supported**: 5

**[📄 Full Details](PHASE2_COMPLETE.md)**

---

## ✅ Phase 3: MQTT Gateway (COMPLETE)

**Duration**: 2 months (May-Jun 2024)
**Status**: ✅ Complete
**Completion**: 100%

### Objectives
Build a complete bidirectional MQTT gateway with message transformation.

### Deliverables
- ✅ MQTT broker client (rumqttc)
- ✅ UDP listener (port 11884)
- ✅ Subscription management (INI config)
- ✅ Message transformer pipeline
- ✅ Built-in transformers (JSON expansion, boolean conversion)
- ✅ External script support (Perl/PHP/Bash)
- ✅ Bidirectional relay (MQTT ↔ Miniserver)
- ✅ Hot-reload capability
- ✅ Broadcast channel for real-time UI

### Key Metrics
- **Lines of Code**: 2,500+
- **Transformers**: 2 built-in + unlimited external
- **Throughput**: 1000+ messages/sec
- **Latency**: <50ms

**[📄 Full Details](PHASE3_COMPLETE.md)**

---

## ✅ Phase 4: Web UI (COMPLETE)

**Duration**: 3 months (Jul-Sep 2024)
**Status**: ✅ Complete
**Completion**: 100%

### Objectives
Create a modern server-rendered web interface with real-time capabilities.

### Deliverables
- ✅ Askama template engine integration
- ✅ HTMX for progressive enhancement
- ✅ Dashboard with system overview
- ✅ Miniserver management (CRUD)
- ✅ Real-time MQTT monitor (SSE)
- ✅ 4-tab MQTT configuration interface:
  - Broker settings
  - Subscriptions with RegEx filters
  - Conversions/transformers
  - Incoming messages viewer
- ✅ Plugin management UI
- ✅ Settings page
- ✅ Professional branding (favicon, logo)
- ✅ Responsive CSS design

### Key Metrics
- **Lines of Code**: 2,000+
- **Templates**: 8 HTML files
- **Pages**: 6 main pages
- **Real-time Features**: SSE streaming
- **Load Time**: <100ms

**[📄 Full Details](PHASE4_COMPLETE.md)**

---

## ✅ Phase 5: SDK Compatibility & Logging (COMPLETE)

**Duration**: 6 weeks (Q1 2025)
**Status**: ✅ Complete
**Completion**: 100%
**Released**: v1.2.0

### Objectives
Enable full compatibility with existing LoxBerry plugins and implement production-grade logging.

### Deliverables
- ✅ SDK compatibility layer (Perl/PHP/Bash)
- ✅ Environment variable injection for plugins
- ✅ Plugin execution wrapper
- ✅ Structured logging with tracing
- ✅ Log rotation with tracing-appender
- ✅ Per-component log levels
- ✅ Plugin-specific log files
- ✅ Web UI log viewer
- ✅ Backup & restore functionality
- ✅ Configuration validation

### Key Metrics
- **Plugins Compatible**: 3+ real LoxBerry plugins tested (incl. Vitoconnect)
- **SDK Coverage**: Perl, PHP, Bash layers complete

**[📄 Full Details](PHASE5_COMPLETE.md)**

---

## ✅ Phase 6: Performance & Monitoring (COMPLETE)

**Duration**: 6 weeks (Q2 2025)
**Status**: ✅ Complete
**Completion**: 100%
**Released**: v1.3.0

### Objectives
Make the system production-ready with monitoring, observability, and operational tooling.

### Deliverables
- ✅ Database abstraction layer (PostgreSQL / SQLite)
- ✅ Email notification system (SMTP + HTML templates)
- ✅ Task scheduler with cron expression support
- ✅ Network diagnostics (ping, port scan, connectivity tests)
- ✅ System health monitoring (CPU, memory, disk usage)
- ✅ Backup and restore functionality
- ✅ Enhanced health check endpoint with detailed metrics

**[📄 Full Details](PHASE6_COMPLETE.md)**

---

## ✅ Phase 7: Security Hardening (COMPLETE)

**Duration**: 8 weeks (Q3 2025)
**Status**: ✅ Complete
**Completion**: 100%
**Released**: v1.3.0

### Objectives
Production-grade security for the RustyLox platform.

### Deliverables

#### Authentication & Authorization
- ✅ JWT authentication (HS256)
- ✅ Role-Based Access Control (RBAC) — Admin, Operator, Viewer, PluginManager
- ✅ API key management (`lbx_` prefix, SHA-256 hashing)
- ✅ Argon2id password hashing
- ✅ Account lockout after 5 failed login attempts
- ✅ In-memory session management with auto-expiry
- ✅ Default admin user created on first run

#### Security Infrastructure
- ✅ Security headers middleware (CSP, X-Frame-Options, etc.)
- ✅ Comprehensive audit logging for all security-sensitive operations
- ✅ Auth REST API (`/api/auth/*`, `/api/users/*`)

### Key Metrics
- **Auth endpoints**: JWT + API key dual-mode
- **Password hashing**: Argon2id (industry best practice)
- **Audit log**: Full trail for compliance

**[📄 Full Details](PHASE7_COMPLETE.md)**

---

## ✅ Phase 7a: Complete Web UI for Backend Features (COMPLETE)

**Duration**: 6 weeks (Q1 2026)
**Status**: ✅ Complete
**Completion**: 100%

### Objectives
Build a full web UI for all backend functionality introduced in phases 5–7, and expand Miniserver backup capabilities.

### Deliverables
- ✅ Login/logout with JWT cookie authentication
- ✅ User management UI (`/admin/users`)
- ✅ API key management UI (`/admin/api-keys`)
- ✅ Audit log viewer (`/admin/audit`)
- ✅ Security settings UI (`/admin/security`)
- ✅ Database management UI (`/admin/database`)
- ✅ System health dashboard (`/system-health`)
- ✅ Email configuration UI (`/email`) — loads real config
- ✅ Task scheduler UI (`/tasks`)
- ✅ Network diagnostics UI (`/network`)
- ✅ Backup & restore UI (`/backup`)
- ✅ Miniserver backup — full recursive backup of all data directories via SSE progress bar
- ✅ LoxBerry-compatible log viewer route (`/admin/system/tools/logfile.cgi`)
- ✅ Weather widget + API docs (bonus pages)
- ✅ System update UI — check GitHub releases, view release notes, update instructions (`/system-update`)
- ✅ Email send history viewer — persisted JSON history with status badges
- ✅ Task execution history viewer — file-persisted execution history
- ✅ CSS/responsive polish and accessibility — skip-link, focus-visible, mobile nav, print styles

**[📄 Full Details](PHASE7A_COMPLETE.md)**

---

## 📅 Phase 8: Advanced Features & Ecosystem (FUTURE)

**Duration**: 12 weeks (Q2–Q3 2026)
**Status**: 📅 Future
**Completion**: 0%
**Priority**: MEDIUM

### Objectives
Expand the platform with advanced integrations, developer tooling, and cloud-ready deployment options.

### Planned Deliverables

#### Cloud & High Availability
- [ ] Kubernetes manifests
- [ ] Redis session storage + caching layer
- [ ] Load balancing (nginx)
- [ ] Multi-instance support

#### Integrations
- [ ] OAuth2 (Google, GitHub, Azure AD)
- [ ] Home Assistant bridge
- [ ] Voice assistant integration (Alexa, Google Home)
- [ ] Prometheus metrics exporter (50+ metrics)

#### Developer Experience
- [ ] Plugin development CLI
- [ ] API playground (Swagger UI)
- [ ] Plugin documentation generator
- [ ] Testing framework for plugins
- [ ] Rust SDK for new plugins

**[📋 View Plan](PHASE8_PLAN.md)**

---

## Beyond Phase 8: Future Ideas

### Plugin Ecosystem
- Plugin marketplace integration
- Plugin version management
- Plugin dependencies
- Plugin sandboxing
- Rust SDK for new plugins

### Advanced Features
- Multi-tenancy support
- GraphQL API (alongside REST)
- WebAssembly plugin support
- Mobile app (iOS/Android)
- Voice assistant integration (Alexa, Google Home)
- Home Assistant integration
- Dashboard widgets system
- Notification templates

### Developer Tools
- Plugin development CLI
- Testing framework for plugins
- Plugin documentation generator
- Live debugging console
- API playground (Swagger UI)

---

## Success Metrics

### Overall Project Goals

| Metric | Target | Current |
|--------|--------|---------|
| **Code Quality** | A grade | A |
| **Test Coverage** | >80% | 60% |
| **Performance** | <100ms avg | 50ms |
| **Docker Image Size** | <200MB | 150MB |
| **Plugin Compatibility** | >50 plugins | 3 tested |
| **Documentation** | Complete | 70% |
| **Security** | A+ grade | B+ |
| **Uptime** | >99.9% | N/A |

### Technology Stack Quality

- ✅ **Rust**: 1.80+ (Edition 2021)
- ✅ **Async Runtime**: Tokio (production-ready)
- ✅ **Web Framework**: Axum 0.7 (modern)
- ✅ **MQTT**: rumqttc (reliable)
- ✅ **Templates**: Askama (type-safe)
- ✅ **Testing**: cargo test (comprehensive)
- ✅ **CI/CD**: GitHub Actions (automated)

---

## Contributing to the Roadmap

We welcome suggestions for the roadmap! If you have ideas:

1. **Open an Issue**: Describe your feature request
2. **Join Discussions**: Participate in roadmap discussions
3. **Vote on Features**: React with 👍 on issues you'd like to see
4. **Submit PRs**: Implement features from the roadmap

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

## Version History

| Version | Date | Phase | Major Features |
|---------|------|-------|----------------|
| 0.1.0 | 2024-01 | 1 | Initial scaffolding |
| 1.0.0 | 2024-09 | 1-4 | Complete core system |
| 1.2.0 | 2025-03 | 5 | SDK compatibility, logging, MQTT UI |
| 1.3.0 | 2026-03 | 6-7 | Monitoring, security hardening, JWT/RBAC |
| 0.6.x | 2026-03 | 7a | Miniserver backup w/ SSE progress, log viewer, CI improvements |
| 1.4.0 | 2026-Q2 | 7a | Complete web UI, system update, CSS polish (planned) |
| 2.0.0 | 2026-Q3 | 8 | Advanced features & ecosystem (planned) |

---

## Quick Links

- 📖 **[README.md](README.md)** - Project overview
- 📝 **[CHANGELOG.md](CHANGELOG.md)** - Version history
- 🤝 **[CONTRIBUTING.md](CONTRIBUTING.md)** - How to contribute
- 📜 **[LICENSE](LICENSE)** - Apache 2.0

### Phase Documentation
- ✅ **[Phase 1](PHASE1_COMPLETE.md)** - Foundation
- ✅ **[Phase 2](PHASE2_COMPLETE.md)** - Plugin System
- ✅ **[Phase 3](PHASE3_COMPLETE.md)** - MQTT Gateway
- ✅ **[Phase 4](PHASE4_COMPLETE.md)** - Web UI
- ✅ **[Phase 5](PHASE5_COMPLETE.md)** - SDK & Logging
- ✅ **[Phase 6](PHASE6_COMPLETE.md)** - Performance & Monitoring
- ✅ **[Phase 7](PHASE7_COMPLETE.md)** - Security Hardening
- ✅ **[Phase 7a](PHASE7A_COMPLETE.md)** - Web UI for Backend Features
- 📅 **[Phase 8](PHASE8_PLAN.md)** - Advanced Features & Ecosystem

---

<div align="center">

**Made with ❤️ by the RustyLox Community**

[![GitHub](https://img.shields.io/badge/GitHub-RustyLox-blue)](https://github.com/boernmaster/RustyLox)
[![Docker](https://img.shields.io/badge/Docker-ghcr.io-brightgreen)](https://ghcr.io/boernmaster/rustylox)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

</div>
