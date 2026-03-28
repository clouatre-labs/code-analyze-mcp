# OpenSSF Best Practices Badge -- Passing Level Assessment

**Project:** code-analyze-mcp
**Repository:** <https://github.com/clouatre-labs/code-analyze-mcp>
**Criteria source:** <https://www.bestpractices.dev/en/criteria/0> (retrieved 2026-03-26)
**Badge application:** <https://www.bestpractices.dev/en/projects/new>

---

## How to Read This Document

Each criterion is listed with its canonical ID (e.g., `description_good`), a
strength indicator in brackets -- **MUST** (required), **SHOULD** (recommended),
**SUGGESTED** (optional) -- and one of:

| Status | Meaning |
|--------|---------|
| **Met** | Evidence exists in the repository |
| **Not Met** | Gap exists; action required |
| **N/A** | Criterion explicitly does not apply to this project |

---

## 1. Basics

### 1.1 Basic project website content

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `description_good` | **MUST** -- Website succinctly describes what the software does. | **Met** | README opens with "Standalone MCP server for code structure analysis using tree-sitter." Crates.io description matches. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |
| `interact` | **MUST** -- Website provides information on how to obtain, give feedback, and contribute. | **Met** | README has Installation section (Homebrew, cargo-binstall, cargo install), links CONTRIBUTING.md, SECURITY.md, and issue tracker. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |
| `contribution` | **MUST** -- Contribution information explains the contribution process. | **Met** | CONTRIBUTING.md documents fork/PR workflow, commit format, and PR checklist. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |
| `contribution_requirements` | **SHOULD** -- Contribution information includes requirements for acceptable contributions. | **Met** | CONTRIBUTING.md specifies coding standard (clippy -D warnings, cargo fmt), commit signing (GPG + DCO), and PR checklist. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |

### 1.2 FLOSS license

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `floss_license` | **MUST** -- Software produced by the project MUST be released as FLOSS. | **Met** | Apache-2.0 license; OSI-approved. LICENSE file present, `license = "Apache-2.0"` in Cargo.toml. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/LICENSE |
| `floss_license_osi` | **SUGGESTED** -- License approved by the Open Source Initiative. | **Met** | Apache-2.0 is on the OSI approved list. URL: https://opensource.org/license/apache-2-0 |
| `license_location` | **MUST** -- License posted in a standard location in the source repository. | **Met** | `LICENSE` file at repository root. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/LICENSE |

### 1.3 Documentation

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `documentation_basics` | **MUST** -- Project provides basic documentation for the software. | **Met** | README documents installation, quick-start, MCP client configuration, and all four tools with examples. docs/ directory contains ARCHITECTURE.md, DESIGN-GUIDE.md, OBSERVABILITY.md, ROADMAP.md. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |
| `documentation_interface` | **MUST** -- Project provides reference documentation describing the external interface (input and output). | **Met** | README documents all four MCP tool parameters and output formats with worked examples. AGENTS.md provides a quick-reference parameter table. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |

### 1.4 Other

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `sites_https` | **MUST** -- Project sites support HTTPS. | **Met** | GitHub (https://github.com/clouatre-labs/code-analyze-mcp), crates.io (https://crates.io/crates/code-analyze-mcp), Homebrew tap all use HTTPS. URL: https://github.com/clouatre-labs/code-analyze-mcp |
| `discussion` | **MUST** -- Project has searchable discussion mechanisms with URL-addressable topics that do not require proprietary client software. | **Met** | GitHub Issues are searchable, URL-addressable, and publicly accessible without proprietary software. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues |
| `english` | **SHOULD** -- Documentation is in English and the project can accept bug reports in English. | **Met** | All documentation is in English; issue tracker accepts reports in English. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |
| `maintained` | **MUST** -- Project is maintained. | **Met** | Active development: v0.1.11 released 2026-03-27, dozens of issues closed in March 2026, Renovate bot running weekly. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.1.11 |

---

## 2. Change Control

### 2.1 Public version-controlled source repository

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `repo_public` | **MUST** -- Project has a publicly readable version-controlled source repository with a URL. | **Met** | Public GitHub repository at https://github.com/clouatre-labs/code-analyze-mcp. URL: https://github.com/clouatre-labs/code-analyze-mcp |
| `repo_track` | **MUST** -- Repository tracks what changed, who changed it, and when. | **Met** | Git history records author, timestamp, and GPG signature for every commit. URL: https://github.com/clouatre-labs/code-analyze-mcp/commits/main |
| `repo_interim` | **MUST** -- Repository includes interim versions for review between releases; not only final releases. | **Met** | Feature branches and PR commits are visible in the public repository before merge. URL: https://github.com/clouatre-labs/code-analyze-mcp/pulls |
| `repo_distributed` | **SUGGESTED** -- Common distributed version control software (e.g., git) is used. | **Met** | Git via GitHub. URL: https://github.com/clouatre-labs/code-analyze-mcp |

### 2.2 Unique version numbering

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `version_unique` | **MUST** -- Project results have a unique version identifier for each release. | **Met** | Versions follow 0.1.0 through 0.1.11; Cargo.toml version must match release tag (enforced in release workflow). URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `version_semver` | **SUGGESTED** -- SemVer or CalVer format is used. | **Met** | CONTRIBUTING.md explicitly states "We follow SemVer: MAJOR (breaking), MINOR (features), PATCH (fixes)." URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |
| `version_tags` | **SUGGESTED** -- Releases are identified within the version control system (e.g., git tags). | **Met** | Every release has a GPG-signed annotated git tag (e.g., `v0.1.11`); the release workflow verifies the tag signature before building. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.1.11 |

### 2.3 Release notes

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `release_notes` | **MUST** -- Each release provides human-readable release notes summarizing major changes. | **Met** | Every GitHub release has curated release notes (e.g., v0.1.11) with categorized sections (New Features, Performance, Fixes, CI/Chore). URL: https://github.com/clouatre-labs/code-analyze-mcp/releases |
| `release_notes_vulns` | **MUST** -- Release notes identify every publicly known run-time vulnerability fixed in that release that had a CVE assignment. | **Met** | No CVE-assigned vulnerabilities have been fixed to date. The criterion is satisfied when there is nothing to report; if a CVE is fixed in a future release, it must be listed explicitly. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases |

---

## 3. Reporting

### 3.1 Bug-reporting process

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `report_process` | **MUST** -- Project provides a process for users to submit bug reports. | **Met** | GitHub Issues are enabled with a structured bug report template (`.github/ISSUE_TEMPLATE/bug.md`). README links SECURITY.md for sensitive reports. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues |
| `report_tracker` | **SHOULD** -- Project uses an issue tracker for tracking individual issues. | **Met** | GitHub Issues used as the primary tracker; Renovate and Copilot bots also create tracked issues. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues |
| `report_responses` | **MUST** -- Project acknowledges a majority of bug reports submitted in the last 2--12 months. | **Met** | All bug-labeled issues examined show timely responses and closure (e.g., #429, #418, #415, #368, #365 all closed with fixes). URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Abug+is%3Aclosed |
| `enhancement_responses` | **SHOULD** -- Project responds to a majority (>50%) of enhancement requests in the last 2--12 months. | **Met** | Feature and enhancement issues are tracked and closed (e.g., #442 -- LRU cache extension -- closed with implementation). URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Aenhancement+is%3Aclosed |
| `report_archive` | **MUST** -- Project has a publicly available archive for reports and responses. | **Met** | GitHub Issues are publicly readable and searchable indefinitely. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues |

### 3.2 Vulnerability report process

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `vulnerability_report_process` | **MUST** -- Project publishes the process for reporting vulnerabilities on the project site. | **Met** | SECURITY.md documents the vulnerability reporting process at the repository root. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |
| `vulnerability_report_private` | **MUST** (if private reports supported) -- Project includes instructions for sending reports privately. | **Met** | SECURITY.md instructs reporters to use GitHub's private vulnerability reporting advisory form. Private vulnerability reporting is enabled on the repository (Settings → Security → Private vulnerability reporting). URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |
| `vulnerability_report_response` | **MUST** -- Initial response time for vulnerability reports received in the last 6 months is ≤ 14 days. | **Met** | The repository is actively maintained with same-day or next-day turnaround on security-labeled issues. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Asecurity+is%3Aclosed |

---

## 4. Quality

### 4.1 Working build system

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `build` | **MUST** -- Project provides a working build system that can automatically rebuild from source. | **Met** | `cargo build --release` produces the binary. Documented in README ("Build from source" section). URL: https://github.com/clouatre-labs/code-analyze-mcp#installation |
| `build_common_tools` | **SUGGESTED** -- Common tools are used for building. | **Met** | Cargo is the standard Rust build tool. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `build_floss_tools` | **SHOULD** -- Software is buildable using only FLOSS tools. | **Met** | Rust toolchain, Cargo, and all dependencies are open source. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |

### 4.2 Automated test suite

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `test` | **MUST** -- Project uses at least one automated test suite released as FLOSS, and documents how to run it. | **Met** | Rust's built-in test framework (`cargo test`) is used. 11 test files in `tests/` covering acceptance, integration, semantic correctness, idempotency, MCP smoke tests, and output size. CONTRIBUTING.md documents `cargo test`. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/tests |
| `test_invocation` | **SHOULD** -- Test suite is invocable in a standard way for that language. | **Met** | `cargo test` is the standard Rust invocation. CI runs it on every PR. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |
| `test_most` | **SUGGESTED** -- Test suite covers most code branches, input fields, and functionality. | **Met** | Test files cover all four MCP tools, multiple languages, edge cases (empty files, large files, pagination, summary mode, cursor), and idempotency. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/tests |
| `test_continuous_integration` | **SUGGESTED** -- Project implements continuous integration. | **Met** | GitHub Actions CI runs on every push to main and every pull request: format, lint (clippy), test, dependency audit, zizmor workflow security scan, and commitlint. URL: https://github.com/clouatre-labs/code-analyze-mcp/actions |

### 4.3 New functionality testing

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `test_policy` | **MUST** -- Project has a general policy that new major functionality should be accompanied by tests in the automated test suite. | **Met** | PR template includes "Test Plan" section and a checklist item "Tests pass: `cargo test`". CONTRIBUTING.md lists `test` as a recognized commit type for "adding missing tests or correcting existing tests". URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md |
| `tests_are_added` | **MUST** -- Evidence that the test policy has been adhered to in recent major changes. | **Met** | PR template requires a Test Plan. Recent PRs (e.g., #436 fields projection, #435 impl_only, #443 LRU cache) each have associated tests visible in `tests/`. URL: https://github.com/clouatre-labs/code-analyze-mcp/pulls?q=is%3Aclosed |
| `tests_documented_added` | **SUGGESTED** -- The policy on adding tests is documented in the instructions for change proposals. | **Met** | PR template explicitly includes a "Test Plan" section. CONTRIBUTING.md PR checklist includes `[ ] Tests pass (cargo test)`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md |

### 4.4 Warning flags

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `warnings` | **MUST** -- Project enables compiler warning flags, a safe language mode, or a linter. | **Met** | `cargo clippy -- -D warnings -W clippy::cognitive_complexity` runs on every PR (CI `lint` job). URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `warnings_fixed` | **MUST** -- Project addresses warnings. | **Met** | `-D warnings` promotes all Clippy warnings to errors, preventing merging of any PR with unresolved warnings. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `warnings_strict` | **SUGGESTED** -- Projects are maximally strict with warnings where practical. | **Met** | `-D warnings` is maximally strict: zero warnings permitted. `clippy.toml` sets `cognitive-complexity-threshold = 25`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/clippy.toml |

---

## 5. Security

### 5.1 Secure development knowledge

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `know_secure_design` | **MUST** -- At least one primary developer knows how to design secure software. | **Met** | Evidence: zizmor workflow security scanning, SHA-pinned GitHub Actions, cosign artifact signing, SLSA provenance attestations, branch protection with signed commits, cargo deny for advisory/license checks. These controls reflect applied secure development knowledge. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `know_common_errors` | **MUST** -- At least one primary developer knows of common vulnerability types and mitigations for this kind of software. | **Met** | SECURITY.md labels issues with `security` label; recent security issues (#483--486) address `unwrap()` in hot paths, depth caps on recursive AST traversal, and mutex poisoning -- all common Rust vulnerability classes. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Asecurity+is%3Aclosed |

### 5.2 Use basic good cryptographic practices

The software produced by this project (a code analysis MCP server) does not implement cryptography. Crypto criteria are N/A or trivially met as noted.

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `crypto_published` | **MUST** -- Software uses only publicly published, expert-reviewed cryptographic protocols by default (if crypto is used). | **N/A** | Project does not implement or invoke cryptographic protocols. |
| `crypto_call` | **SHOULD** -- Application calls only software specifically designed to implement cryptographic functions (if crypto is used). | **N/A** | Project does not use cryptography. |
| `crypto_floss` | **MUST** -- All crypto-dependent functionality is implementable using FLOSS. | **N/A** | Project does not use cryptography. |
| `crypto_keylength` | **MUST** -- Default key lengths meet NIST minimums through 2030. | **N/A** | Project does not use cryptographic keys. |
| `crypto_working` | **MUST** -- Default security mechanisms do not depend on broken cryptographic algorithms. | **N/A** | Project does not use cryptographic algorithms. |
| `crypto_weaknesses` | **SHOULD** -- Default mechanisms do not use algorithms with known serious weaknesses. | **N/A** | Project does not use cryptography. |
| `crypto_pfs` | **SHOULD** -- Key agreement protocols implement perfect forward secrecy. | **N/A** | Project does not implement key agreement protocols. |
| `crypto_password_storage` | **MUST** -- Passwords stored as iterated hashes with per-user salt (if project stores passwords). | **N/A** | Project does not store passwords. |
| `crypto_random` | **MUST** -- Cryptographic keys and nonces use a CSPRNG. | **N/A** | Project does not generate cryptographic keys or nonces. |

### 5.3 Secured delivery against man-in-the-middle (MITM) attacks

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `delivery_mitm` | **MUST** -- Project uses a delivery mechanism that counters MITM attacks. | **Met** | All distribution channels use HTTPS: GitHub Releases (HTTPS), crates.io (HTTPS), Homebrew tap (HTTPS). Binaries are additionally signed with cosign and have GitHub SLSA provenance attestations. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases |
| `delivery_unsigned` | **MUST** -- Cryptographic hashes must not be retrieved over HTTP and used without signature verification. | **Met** | All download URLs use HTTPS. Per-binary `.sha256` files are published alongside release tarballs on the same HTTPS-protected GitHub Releases page. cosign `.bundle` signatures enable independent verification. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.1.11 |

### 5.4 Publicly known vulnerabilities fixed

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `vulnerabilities_critical_fixed` | **MUST** -- No unpatched vulnerabilities of medium or higher severity for more than 60 days. | **Met** | `cargo deny check advisories` runs on every PR and Renovate PR, blocking merges if known vulnerabilities are present. No open CVEs in the dependency tree as of the latest audit. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/deny.toml |
| `vulnerabilities_critical_fixed` (rapid fix) | **SHOULD** -- Critical vulnerabilities fixed rapidly after reporting. | **Met** | Security-labeled issues (#483--486) were created and closed within the same release cycle. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Asecurity+is%3Aclosed |

### 5.5 Other security issues

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `no_leaked_credentials` | **MUST** -- Public repositories must not leak valid private credentials. | **Met** | No credentials, API keys, or secrets in the repository. Zizmor workflow security scanning would flag secret leaks. GitHub secret scanning is not enabled, but the project has no secrets to leak (no external service credentials in CI; GITHUB_TOKEN is ephemeral). URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/zizmor.yml |

---

## 6. Analysis

### 6.1 Static code analysis

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `static_analysis` | **MUST** -- At least one static analysis tool applied to proposed major production releases, if a FLOSS tool exists. | **Met** | `cargo clippy -- -D warnings` runs on every PR (CI `lint` job). `cargo deny check advisories licenses` also runs. Zizmor scans GitHub Actions workflows. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `static_analysis_common_vulnerabilities` | **SUGGESTED** -- At least one static analysis tool looks for common vulnerabilities in the analyzed language. | **Met** | Clippy includes lints for common Rust vulnerabilities (integer overflows, unwrap in non-test code, etc.). cargo deny checks known CVE advisories. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/deny.toml |
| `static_analysis_fixed` | **MUST** -- Medium+ severity exploitable vulnerabilities found by static analysis are fixed in a timely way. | **Met** | `-D warnings` means no clippy warning can be merged. cargo deny blocks PRs with known advisory matches. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `static_analysis_often` | **SUGGESTED** -- Static source code analysis occurs on every commit or at least daily. | **Met** | CI runs clippy and cargo deny on every pull request and on every push to main. Renovate runs weekly and triggers the full CI suite. URL: https://github.com/clouatre-labs/code-analyze-mcp/actions |

### 6.2 Dynamic code analysis

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `dynamic_analysis` | **SUGGESTED** -- At least one dynamic analysis tool applied to proposed major production releases. | **Met** | A cargo-fuzz harness is present in [`fuzz/`](../fuzz/) with three targets exercising the main entry points via libFuzzer. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/fuzz |
| `dynamic_analysis_unsafe` | **SUGGESTED** -- If the project uses memory-unsafe languages, at least one dynamic tool detects memory safety problems. | **N/A** | The project is written entirely in Rust (memory-safe by design). No C or C++ code is produced. |
| `dynamic_analysis_enable_assertions` | **SUGGESTED** -- Dynamic analysis configuration enables many assertions. | **Met** | The `[profile.fuzz]` section in `Cargo.toml` sets `debug-assertions = true`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `dynamic_analysis_fixed` | **MUST** -- Medium+ severity exploitable vulnerabilities discovered by dynamic analysis are fixed in a timely way. | **Met** | The fuzz harness is in place. Any vulnerability found during fuzzing would follow the same security disclosure and fix process documented in SECURITY.md. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |

---

## Summary

### Counts

| Category | Met | Not Met | N/A |
|----------|-----|---------|-----|
| Basics | 13 | 0 | 0 |
| Change Control | 8 | 0 | 0 |
| Reporting | 6 | 0 | 0 |
| Quality | 11 | 0 | 0 |
| Security -- Knowledge | 2 | 0 | 0 |
| Security -- Crypto | 0 | 0 | 9 |
| Security -- Delivery | 2 | 0 | 0 |
| Security -- Vulns | 2 | 0 | 0 |
| Security -- Other | 1 | 0 | 0 |
| Analysis | 7 | 0 | 0 |
| **Total** | **52** | **0** | **9** |

### Required Actions (MUST criteria not yet met)

All MUST criteria are met. No required actions remain.

### Recommended Actions (SHOULD / SUGGESTED criteria not yet met)

All SUGGESTED criteria are now met. No recommended actions remain.

### Badge

The project is registered at https://www.bestpractices.dev/projects/12275.
The badge is displayed in README.md and SECURITY.md.
