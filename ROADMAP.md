# RustyLox Roadmap

<div align="center">

![Project Status](https://img.shields.io/badge/Status-Production%20Ready-brightgreen)
![Version](https://img.shields.io/badge/Current-v0.8.2-blue)

</div>

## Vision

Transform LoxBerry into a modern, secure, and scalable smart home platform built with Rust, while maintaining backward compatibility with the existing plugin ecosystem.

---

## Current State

RustyLox is production-ready at **v0.8.2**. The full core platform is implemented and stable:

| Area | Status | Details |
|------|--------|---------|
| Core types & config | Complete | JSON-based config, Miniserver HTTP/UDP client |
| Plugin system | Complete | ZIP install/uninstall, lifecycle hooks, SDK compatibility (Perl/PHP/Bash) |
| MQTT gateway | Complete | Broker integration, UDP listener, transformer pipeline, hot-reload |
| Web UI | Complete | Askama + HTMX, dashboard, MQTT monitor, plugin management, admin panel |
| Logging & SDK | Complete | Structured logging, log rotation, plugin execution wrapper |
| Monitoring | Complete | Database abstraction, email (SMTP), task scheduler, network diagnostics, backup/restore |
| Security | Complete | JWT auth, RBAC, API keys, Argon2id, account lockout, audit log, security headers |
| Admin UI | Complete | User management, API key UI, audit log viewer, system health, email/task history |

---

## Version History

| Version | Released | Highlights |
|---------|----------|-----------|
| 0.1.0 | 2024-01 | Initial scaffolding |
| 1.0.0 | 2024-09 | Complete core system (config, Miniserver, MQTT, Web UI) |
| 1.2.0 | 2025-03 | SDK compatibility, structured logging, MQTT subscriptions UI |
| 1.3.0 | 2026-03 | Monitoring, task scheduler, email, JWT/RBAC security hardening |
| 0.6.x | 2026-03 | Miniserver backup with SSE progress, log viewer, CI improvements |
| 0.7.0 | 2026-03 | Admin UI, system update page, email/task history, CSS polish, accessibility |
| 0.8.0 | 2026-03 | MQTT nav refactor; Incoming Overview & MQTT Finder as tabs on /mqtt/config |
| 0.8.2 | 2026-03 | Native weather service, CI multi-arch builds, dnsmasq DNS redirect |

---

## Next: Advanced Features & Ecosystem

Planned work beyond the current production-ready baseline:

### High Priority
- **Plugin marketplace** — central registry for discovering and installing plugins
- **Kubernetes deployment** — manifests and Helm chart
- **OAuth2 / OIDC** — single sign-on with Google, GitHub, Azure AD
- **Progressive Web App (PWA)** — mobile-friendly installable app
- **Plugin sandboxing** — container-based isolation with resource limits

### Medium Priority
- **2FA (TOTP)** — Google Authenticator / Authy support
- **High availability** — multi-instance support, Redis session storage
- **OpenTelemetry tracing** — distributed request tracing
- **Plugin auto-updates** — automatic update checks and rollback
- **GraphQL API** — alongside existing REST API

### Lower Priority
- **Multi-tenancy** — isolated tenants in a single instance
- **Voice assistant integration** — Alexa, Google Home
- **Smart home protocol support** — Zigbee/Z-Wave/Matter via integrations
- **Enterprise compliance** — GDPR tooling, SOC 2 reporting, SLA monitoring

---

## Success Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Code quality | A grade | A |
| Test coverage | >80% | ~60% |
| API response time | <100ms avg | ~50ms |
| Docker image size | <200MB | ~150MB |
| Plugin compatibility | >50 plugins | 3+ tested |
| Security grade | A+ | B+ |

---

## Contributing to the Roadmap

We welcome suggestions! If you have ideas:

1. **Open an Issue** — describe your feature request
2. **Join Discussions** — participate in roadmap discussions
3. **Vote on Features** — react with +1 on issues you want to see
4. **Submit PRs** — implement features from the roadmap

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

## Quick Links

- **[README.md](README.md)** — Project overview and feature list
- **[CHANGELOG.md](CHANGELOG.md)** — Version history
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — How to contribute

---

<div align="center">

[![GitHub](https://img.shields.io/badge/GitHub-RustyLox-blue)](https://github.com/boernmaster/RustyLox)
[![Docker](https://img.shields.io/badge/Docker-ghcr.io-brightgreen)](https://ghcr.io/boernmaster/rustylox)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

</div>
