# Repository Standards

## Overview

This document codifies the conventions and principles that govern `code-analyze-mcp` at the repository level: how we organize issues, structure CI/CD, manage releases, and maintain documentation. These standards serve three purposes: they enable fast feedback during development, they ensure security and quality at release time, and they provide a reproducible template for sibling repositories like `code-edit-mcp`. This document is the canonical reference for repository setup; it complements `ARCHITECTURE.md` (which covers component design) and the [MCP agents orchestration guide](https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/anthropic-mcp-agents-orchestration.md) (which documents MCP server patterns and integration with Claude agents).

## Repository Metadata

The repository is tagged with four topics: `ai`, `code-analysis`, `developer-tools`, and `tree-sitter`. These tags signal the intended audience and primary use case; they are searchable on GitHub and help onboard new users quickly.

Eleven labels span the full lifecycle of issues and PRs: `bug`, `chore`, `ci`, `dependencies`, `documentation`, `enhancement`, `performance`, `refactor`, `release`, `security`, and `testing`. This taxonomy covers every possible change category and enables filtering by concern type. The colors are semantic: red for `bug` and `security` (urgent), green for `release` (milestones), purple for `performance` (optimization focus), blue for `refactor` and `documentation` (structural), and neutral gray for `chore` (maintenance).

Branch and tag protection is enforced via GitHub Rulesets, the modern approach replacing the legacy branch-protection API. The "Protect main branch" ruleset blocks direct pushes and requires status checks to pass; the "Release Tag Protection" ruleset enforces semantic versioning format (`v*.*.*`) and prevents accidental tag overwrites. Rulesets are preferred because they apply consistently across the organization and admit fine-grained conditions that the legacy API cannot express.

## Issue and PR Templates

Three issue templates guide contributors toward structured reporting: `bug.md`, `feature.md`, and `refactor.md`. Each follows the same principle: structured fields reduce triage friction and surface hidden assumptions early.

Bug reports require a summary, reproduction steps, expected and actual behavior, environment details (Rust version, OS), error logs, a root cause hypothesis, and acceptance criteria for the fix. The root cause field is critical; it forces the reporter to reason about *why* the bug occurred, not just *what* happened. This accelerates triage and often surfaces duplicates or known limitations before a reviewer even opens the issue.

Feature requests ask for context (the user story or pain point), prerequisites (dependencies or blockers), implementation notes with strategy, code examples, API references, acceptance criteria, and scope boundaries. The scope section explicitly states what *won't* be done; this prevents scope creep and keeps the issue focused. Code examples and API references ground the discussion in concrete details, not abstract ideas.

Refactoring issues follow the same structure to ensure that refactors are tracked as first-class work, not hidden in feature PRs.

The PR template enforces consistency: Summary (2-3 sentences on what changed and why), Related Issues (links to closed issues), Changes (file-by-file bullet list), Test Plan (unit, integration, edge cases), and a Verification Checklist. The checklist covers tests, clippy warnings, format compliance, no-unwrap policy, API verification (that new APIs match the installed package version), GPG and DCO signing, scope creep avoidance, and secrets scanning. This checklist is not optional; it is presented at PR creation and must be acknowledged. Skipped items require justification in the PR comment.

## CI Pipeline

The CI pipeline is deliberately lean: it runs only the checks that unblock forward progress. Path-based change detection (via `dorny/paths-filter`) ensures that expensive operations like benchmarks and dependency audits run only when their inputs change. Pushing to `main` with changes to `src/*`, `Cargo.*`, `.github/workflows/*`, or `tests/*` triggers the full pipeline; pushing documentation or non-source files bypasses expensive checks and gives faster feedback.

The pipeline enforces Conventional Commits via `commitlint`; every commit message must follow the `type(scope): description` format. This enables automated changelog generation and makes history searchable.

Each job runs conditionally based on path filters. Linting (format, clippy), unit tests, and benchmarks run only on code changes. The benchmark job runs on `main` only; this prevents PR CI from being blocked by long-running benchmarks while still tracking performance trends. Dependency audits (`cargo deny check advisories licenses`) run on every push to catch security advisories early.

The CI Result is a single aggregate status check that GitHub requires to pass before merging. This is intentional: a single required check is simpler to reason about than a list of ten individual checks, and it reduces alert fatigue. If any step fails, the aggregate fails; the PR author sees a single red X and must investigate the detailed logs.

## Release Pipeline

Releases are triggered by pushing a semantic version tag (`v*.*.*`). The release pipeline verifies the tag signature (GPG), builds binaries for multiple targets (Linux x86-64, macOS aarch64 and x86-64, Windows), signs binaries with cosign, generates provenance attestations via `actions/attest-build-provenance`, and uploads to GitHub Releases.

Provenance attestation is critical for supply chain security. The attestation proves that the binary was built from a specific commit in this repository on a specific date; it is signed by GitHub's authority and cannot be forged offline. Consumers can verify the attestation with `gh attestation verify` before installing.

An SBOM (Software Bill of Materials) is generated and attached to the release. The SBOM lists all dependencies and their versions, enabling security scanners downstream to detect vulnerable transitive dependencies.

Distribution happens via multiple channels: GitHub Releases (direct download), Homebrew (macOS and Linux via `clouatre-labs/homebrew-tools`), `cargo-binstall` (automatic installation of pre-built binaries via `cargo install`), and crates.io (Rust source distribution). Each channel reaches different user segments; the multi-channel strategy maximizes adoption.

Dry-run mode (`workflow_dispatch` with `dry_run: true`) allows testing the release pipeline without publishing. This is invaluable for catching integration bugs before they reach users.

## Security Scanning

The `mcp-scan.yml` workflow performs LLM-based security scanning of the MCP server. It uses the mcp-scanner tool with Claude Haiku to analyze the server implementation for misconfigurations, injection vulnerabilities, and authorization gaps. The scan runs on push to `main`, on release tags, and on pull requests targeting `main`.

Scanning only on release, not on every PR, is a deliberate choice to balance security and CI latency. Full scanning on every PR would slow down feedback and create noise for changes that do not affect security surface. Scans on the merge to `main` and at release time catch problems before they reach users.

## Documentation Structure

The `docs/` directory contains `ARCHITECTURE.md` (component design, module responsibilities, design decisions), `anthropic-mcp-agents-orchestration.md` (MCP protocol patterns and Claude agent integration), `security-testing.md` (threat model and test methodology), `comparison-and-optimization-spec.md` (performance benchmarks and optimization rationale), and `benchmarks/` (versioned results: v3, v4, v5 with methodology, raw results, and analysis).

`ARCHITECTURE.md` is the source of truth for how the code is organized. It defines the module hierarchy (main, lib, analyze, parser, formatter, traversal, cache, graph), the responsibility of each module, and the rationale for major design decisions (why tree-sitter, why parallel processing, why LRU cache with mtime invalidation).

The orchestration guide explains the MCP protocol from a Claude agent perspective: how tools are registered, how arguments flow, how context is managed, and how to integrate new tools. This is upstream documentation, not specific to this repository; it is linked from `ARCHITECTURE.md` and serves onboarding for agent developers.

The repo-standards document (this file) explains *why* each convention exists and is the template for applying standards to sibling repos.

## Build Profiles

Cargo profiles define optimization levels and code generation settings.

The `release` profile is optimized for production binaries: `opt-level=z` (maximum size optimization; the z level does not exist in std Cargo, so this is custom shorthand for aggressive optimization), `lto=true` (link-time optimization to reduce binary size and enable interprocedural optimizations), `codegen-units=1` (single-pass code generation enables better optimizations at the cost of longer compile time), `panic=abort` (reduce binary size by omitting unwinding), and `strip=true` (remove debug symbols). These flags minimize binary size for distribution and maximize execution speed.

The `ci` profile inherits from `release` but overrides two settings: `lto=false` and `codegen-units=16`. Disabling LTO speeds up CI builds without sacrificing correctness (LTO is nice-to-have for binary size, not essential for functionality); increasing codegen-units to 16 parallelizes compilation and reduces wall-clock time. The trade-off is that CI binaries are larger and slightly slower than release binaries, but CI feedback is 2-3x faster.

The `dev` profile is not customized; it uses Cargo defaults. Development builds prioritize compilation speed and debuggability over runtime performance.

## Applying These Standards to a New Repo

Use this checklist when setting up a sibling repository like `code-edit-mcp`:

**GitHub Metadata**
- [ ] Set repository topics to align with project scope (see Repository Metadata above)
- [ ] Create 11 labels with identical names, colors, and descriptions (copy from source repo)
- [ ] Create Rulesets for branch and tag protection (modern approach; see Repository Metadata)

**Issue and PR Templates**
- [ ] Copy `.github/ISSUE_TEMPLATE/bug.md`, `feature.md`, `refactor.md` from source repo
- [ ] Adapt templates to the target repository's scope (e.g., replace "analyze" with "edit" in feature.md)
- [ ] Copy `.github/PULL_REQUEST_TEMPLATE.md` verbatim
- [ ] Update `.github/copilot-instructions.md` with target repo specifics

**CI Workflows**
- [ ] Copy `.github/workflows/ci.yml` and customize path filters for the target repo's file structure
- [ ] Use `CI Result` as the single required status check (see CI Pipeline above)
- [ ] Copy `commitlint.config.js` and `.github/workflows/lint-commits.yml` verbatim
- [ ] Customize benchmark triggers and metrics to target repo concerns

**Release Pipeline**
- [ ] Copy `.github/workflows/build-and-attest.yml` and update targets if needed
- [ ] Copy `.github/workflows/release.yml` and customize for target repo distribution channels
- [ ] Ensure release notes are generated from git tags (see Release Pipeline above)
- [ ] Set up Homebrew formula repository and update release workflow

**Documentation**
- [ ] Create `docs/ARCHITECTURE.md` documenting the target repo's module structure and design decisions
- [ ] Link to `anthropic-mcp-agents-orchestration.md` as the upstream MCP reference
- [ ] Create `docs/repo-standards.md` by copying this file and specializing for the target repo
- [ ] Create `docs/benchmarks/` if the target repo has performance-critical paths

**Cargo Configuration**
- [ ] Copy `Cargo.toml` profile settings: `[profile.release]` and `[profile.ci]` (see Build Profiles above)
- [ ] Update package metadata (name, description, repository URL) to match target repo
- [ ] Ensure `Cargo.toml` includes `cargo-deny` configuration for dependency auditing

**Security and Signing**
- [ ] Ensure contributors are familiar with GPG signing and DCO sign-off (documented in PR template)
- [ ] Set up cosign signing for release binaries (see Release Pipeline above)
- [ ] Enable mcp-scan in release pipeline if the target is an MCP server
