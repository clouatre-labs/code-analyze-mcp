# Contributing to code-analyze-mcp

We welcome contributions! This document covers the essentials.

## Quick Start

```bash
git clone https://github.com/YOUR_USERNAME/code-analyze-mcp.git
cd code-analyze-mcp
cargo build
cargo test
```

## Before Submitting

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **test**: Adding missing tests or correcting existing tests
- **chore**: Changes to build process, dependencies, or tooling

### Examples

```bash
# Feature with scope
git commit -S --signoff -m "feat(analysis): add support for TypeScript generics"

# Bug fix
git commit -S --signoff -m "fix: handle empty source files without panicking"

# Breaking change
git commit -S --signoff -m "feat!: redesign tool parameter schema

BREAKING CHANGE: The path parameter is now required"

# Documentation
git commit -S --signoff -m "docs: update tool descriptions in README"
```

## Developer Certificate of Origin (DCO)

All commits must be signed off to certify you have the right to submit the code:

```bash
git commit -S --signoff -m "Your commit message"
```

This adds `Signed-off-by: Your Name <email>` to your commit, certifying you agree to the [DCO](https://developercertificate.org/).

The `-S` flag GPG-signs the commit (required by branch protection).

## Pull Request Checklist

- [ ] Tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Commits GPG-signed and signed off (`git commit -S --signoff`)
- [ ] Clear PR description

## Branch Protection

The `main` branch is protected by GitHub rulesets with the following rules:

- **Required Status Checks**: All CI checks must pass before merging
- **Signed Commits**: All commits must be signed (GPG or S/MIME)
- **No Force Push**: History cannot be rewritten on main
- **No Deletion**: The main branch cannot be deleted

Ensure your commits are GPG-signed and all CI checks pass before opening a pull request.

## License

By contributing, you agree your contributions are licensed under [Apache-2.0](LICENSE).
