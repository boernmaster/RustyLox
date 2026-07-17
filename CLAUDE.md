# CLAUDE.md

RustyLox is a Rust + Docker rewrite of [LoxBerry](https://github.com/mschlenstedt/Loxberry), a smart home platform for Loxone systems. It maintains full backward compatibility with existing LoxBerry plugins (Perl/PHP/Bash).

## Knowledge Graph

Architecture, codebase structure, and cross-crate relationships are in `graphify-out/`:
- `graphify-out/graph.html` — interactive visualization
- `graphify-out/GRAPH_REPORT.md` — god nodes, surprising connections

To query: `/graphify query "<question>"` — To update after changes: `/graphify --update`

## Tech Stack

Rust 1.80+ · Axum 0.7 · Askama templates · HTMX · Tokio · rumqttc · JSON file storage (no SQL) · Docker multi-stage

**Crate dependency order (bottom-up)**:
`rustylox-core` → `rustylox-config` → service crates (mqtt-gateway, plugin-manager, miniserver-client, auth, metrics, email-manager, task-scheduler, backup-manager, addon-registry) → `web-api` → `web-ui` → `rustylox-daemon`

## Code Rules

- `cargo fmt` + `cargo clippy` before every commit (CI enforces both)
- No `unwrap()`/`expect()` in production code — use `?`
- No blocking I/O (`std::fs`) in async functions — use `tokio::fs`
- Prefer `tokio::join!` for independent async operations
- All config writes are atomic: write to `.tmp` then `fs::rename`
- No new dependencies without discussion; no SQL/ORM
- No emojis in code

## Git

Conventional commits: `type(scope): description`
Types: `feat` `fix` `docs` `style` `refactor` `test` `chore`

**Before committing**: `cargo fmt && cargo clippy && cargo test`

**Before tagging a release**: update `CHANGELOG.md` first, commit it, then tag. The tag annotation and GitHub Release body must both contain the release notes.

## Non-Obvious Behaviors

- `JWT_SECRET` and `ADMIN_PASSWORD` env vars are **mandatory** — no defaults, daemon refuses to start without them
- MQTT Incoming Overview and MQTT Finder are **tabs on `/mqtt/config`**, not separate pages (since v0.8.0)
- `GET /api/config/general` strips all credential fields before returning (pass_raw, admin_raw, etc.)
- `extract_identity()` (`web-api/src/routes/auth.rs:146`) is called by every protected handler — it's the universal auth guard
- Plugin `preroot.sh`/`postroot.sh` run as the current user, not root — known limitation
- Network interface `is_up` = has at least one IP address assigned (not traffic-based)
- Built-in scheduler tasks (`backup`, `log_rotation`, `health_check`, `miniserver_backup`) are auto-merged on startup — missing ones are added without overwriting existing config
- `data/system/miniserver-backups/` is excluded from system backups
- CSP allows `'unsafe-inline'` — required because 26 inline `<script>` blocks exist in templates
- `libdbi-perl` + `libdbd-sqlite3-perl` are installed in Docker so `LoxBerry::Log` SQLite sessions work
- **Containerized addons** (since v1.3.0, `addon-registry` crate): external processes self-register
  via unauthenticated `POST /api/addons/register` (LAN-trust model, matching the rest of the app),
  are proxied through `addon_registry::proxy` for schema/config/save, and get a generic settings page
  at `/addons/:name/settings` rendered from whatever schema/config JSON the addon returns. The
  addon-proxy HTTP client has redirects explicitly disabled (`Policy::none()`) — this is a deliberate
  SSRF mitigation, not an oversight; don't "fix" a future addon integration bug by re-enabling
  redirect-following without re-reading why it's off (final review, PR #71). Secret-value blanking is
  the *addon's own* responsibility (its `config` response returns `value: ""` + `secret_set` for
  secret fields) — RustyLox's proxy layer is a pure pass-through with no blanking of its own; don't
  assume the web-api layer sanitizes secrets on an addon's behalf. The GHCR catalog client
  (`CatalogClient`, `GET /api/addons/catalog`) is a separate HTTP client that intentionally *does*
  follow redirects (GHCR's blob storage legitimately redirects) — only trusted, hardcoded hosts
  (`api.github.com`, `ghcr.io`), never an addon-controlled URL, go through that client.
