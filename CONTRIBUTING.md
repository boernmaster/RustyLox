# Contributing to RustyLox

## Reporting Bugs

1. Check [existing issues](https://github.com/boernmaster/RustyLox/issues) first
2. Include: Rust version, Docker version, OS, steps to reproduce, expected vs actual behavior, relevant logs

## Suggesting Features

1. Check [ROADMAP.md](ROADMAP.md) — it may already be planned
2. Open a GitHub Discussion before starting a large implementation

## Pull Requests

### Setup

```bash
git clone https://github.com/YOUR_USERNAME/RustyLox.git
cd RustyLox
mkdir -p volumes/config/system volumes/data/system volumes/log/system
cargo build && cargo test
```

See [docs/development.md](docs/development.md) for the full build and test guide.

### Before Submitting

```bash
cargo fmt          # format code
cargo clippy       # fix all warnings
cargo test --all   # all tests must pass
```

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): brief description
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Examples:
```
feat(mqtt): add RegEx filter support for subscriptions
fix(plugin): handle nested ZIP archives correctly
docs(api): document virtual HTTP input endpoint
```

### PR Checklist

- [ ] `cargo fmt` and `cargo clippy` pass with no warnings
- [ ] `cargo test --all` passes
- [ ] New functionality has tests
- [ ] Public API changes are documented
- [ ] PR description explains what and why

## Code Style

- Snake_case for functions and variables, PascalCase for types
- `Result<T>` and `?` for error propagation — no `unwrap()` in library code
- Async functions for I/O, sync for pure computation
- No comments explaining *what* the code does — only *why* when the reason is non-obvious

## License

By contributing, you agree your work will be licensed under [Apache License 2.0](LICENSE).
