# Contributing to RustyLox

<div align="center">

![Contributions Welcome](https://img.shields.io/badge/Contributions-Welcome-brightgreen)
![PRs Welcome](https://img.shields.io/badge/PRs-Welcome-brightgreen)
![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)

</div>

Thank you for your interest in contributing to RustyLox! This document provides guidelines for contributing to the project.

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to uphold this code. Please be respectful and constructive in all interactions.

## How to Contribute

### Reporting Bugs

1. **Check existing issues** first to avoid duplicates
2. **Use the issue template** when creating a new issue
3. **Include details**:
   - Rust version
   - Docker version
   - Operating system
   - Steps to reproduce
   - Expected vs actual behavior
   - Relevant logs

### Suggesting Features

1. **Check the roadmap** to see if it's already planned
2. **Open a discussion** before creating a PR for major features
3. **Be specific** about the use case and benefits

### Pull Requests

#### Before You Start

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/your-feature-name`
3. **Check existing PRs** to avoid duplicate work

#### Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/RustyLox.git
cd RustyLox

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Create volume directories
mkdir -p volumes/config/system volumes/data/system volumes/log/system

# Build and test
cargo build
cargo test

# Run locally
LBHOMEDIR=/tmp/loxberry cargo run --bin loxberry-daemon
```

#### Code Style

- **Run `cargo fmt`** before committing
- **Run `cargo clippy`** and fix warnings
- **Follow Rust conventions**: snake_case for functions/variables, PascalCase for types
- **Add documentation** for public APIs
- **Write tests** for new functionality

#### Commit Messages

Use conventional commits format:

```
type(scope): brief description

Detailed explanation if needed
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Adding tests
- `chore`: Build/tooling changes

Examples:
```
feat(mqtt): add JSON expansion transformer
fix(plugin): handle nested ZIP archives correctly
docs(readme): update installation instructions
```

#### Testing

1. **Write unit tests** for new functions
2. **Write integration tests** for API endpoints
3. **Test with Docker**: `docker compose build && docker compose up -d`
4. **Verify all tests pass**: `cargo test --all`

#### Pull Request Process

1. **Update documentation** if needed (README.md, inline docs)
2. **Add yourself to contributors** if this is your first PR
3. **Create PR with description**:
   - What does this PR do?
   - Why is it needed?
   - How has it been tested?
   - Screenshots (if UI changes)
4. **Link related issues** using "Fixes #123" or "Closes #123"
5. **Wait for review** - maintainers will review within a few days
6. **Address feedback** if requested
7. **Squash commits** if needed before merge

## Project Structure

```
loxberry-rust/
├── crates/
│   ├── loxberry-core/       - Common types and errors
│   ├── loxberry-config/     - JSON config management
│   ├── miniserver-client/   - Miniserver communication
│   ├── mqtt-gateway/        - MQTT Gateway
│   ├── plugin-manager/      - Plugin lifecycle
│   ├── web-api/             - REST API (Axum)
│   ├── web-ui/              - Web UI (Askama + HTMX)
│   └── loxberry-daemon/     - Main binary
├── static/                  - CSS, JS, icons
├── volumes/                 - Docker volume mounts
└── examples/                - Example plugins
```

## Development Workflow

### Adding a New Feature

1. **Create an issue** to discuss the feature
2. **Create a feature branch**
3. **Implement in the appropriate crate**
4. **Add tests**
5. **Update documentation**
6. **Create PR**

### Adding a New API Endpoint

1. **Add route in** `crates/web-api/src/lib.rs`
2. **Add handler in** `crates/web-api/src/routes/`
3. **Add tests in** `crates/web-api/tests/`
4. **Update API documentation in README.md**

### Adding a UI Page

1. **Create template in** `crates/web-ui/templates/`
2. **Add handler in** `crates/web-ui/src/handlers/`
3. **Add route in** `crates/web-ui/src/lib.rs`
4. **Update navigation if needed**

## Questions?

- **GitHub Discussions**: For questions and general discussion
- **GitHub Issues**: For bug reports and feature requests
- **Pull Requests**: For code contributions

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
