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
`rustylox-core` → `rustylox-config` → service crates (mqtt-gateway, plugin-manager, miniserver-client, auth, metrics, email-manager, task-scheduler, backup-manager) → `web-api` → `web-ui` → `rustylox-daemon`

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
