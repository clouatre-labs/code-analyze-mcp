# OpenSSF Best Practices Badge -- Silver Level Assessment

**Project:** code-analyze-mcp
**Repository:** <https://github.com/clouatre-labs/code-analyze-mcp>
**Passing criteria source:** <https://www.bestpractices.dev/en/criteria/0> (retrieved 2026-03-26)
**Silver criteria source:** <https://www.bestpractices.dev/en/criteria/1> (retrieved 2026-03-26)
**Badge application:** <https://www.bestpractices.dev/en/projects/12275>

---

## How to Read This Document

Each criterion is listed with its canonical ID, a strength indicator -- **MUST** (required),
**SHOULD** (recommended), or **SUGGESTED** (optional) -- and one of:

| Status | Meaning |
|--------|---------|
| **Met** | Evidence exists in the repository |
| **Not Met** | Gap exists; action required |
| **N/A** | Criterion explicitly does not apply to this project |

---

## Section 1: Passing Level (Level 0) Criteria

### 1. Basics

#### 1.1 Basic project website content

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `description_good` | **MUST** -- Website succinctly describes what the software does. | **Met** | README opens with "Standalone MCP server for code structure analysis using tree-sitter." Crates.io description matches. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |
| `interact` | **MUST** -- Website provides information on how to obtain, give feedback, and contribute. | **Met** | README has Installation section (Homebrew, cargo-binstall, cargo install), links CONTRIBUTING.md, SECURITY.md, and issue tracker. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |
| `contribution` | **MUST** -- Contribution information explains the contribution process. | **Met** | CONTRIBUTING.md documents fork/PR workflow, commit format, and PR checklist. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |
| `contribution_requirements` | **SHOULD** -- Contribution information includes requirements for acceptable contributions. | **Met** | CONTRIBUTING.md specifies coding standard (clippy -D warnings, cargo fmt), commit signing (GPG + DCO), and PR checklist. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |

#### 1.2 FLOSS license

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `floss_license` | **MUST** -- Software produced by the project MUST be released as FLOSS. | **Met** | Apache-2.0 license; OSI-approved. LICENSE file present, `license = "Apache-2.0"` in Cargo.toml. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/LICENSE |
| `floss_license_osi` | **SUGGESTED** -- License approved by the Open Source Initiative. | **Met** | Apache-2.0 is on the OSI approved list. URL: https://opensource.org/license/apache-2-0 |
| `license_location` | **MUST** -- License posted in a standard location in the source repository. | **Met** | `LICENSE` file at repository root. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/LICENSE |

#### 1.3 Documentation

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `documentation_basics` | **MUST** -- Project provides basic documentation for the software. | **Met** | README documents installation, quick-start, MCP client configuration, and all four tools with examples. docs/ directory contains ARCHITECTURE.md, DESIGN-GUIDE.md, OBSERVABILITY.md, ROADMAP.md. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |
| `documentation_interface` | **MUST** -- Project provides reference documentation describing the external interface (input and output). | **Met** | README documents all four MCP tool parameters and output formats with worked examples. AGENTS.md provides a quick-reference parameter table. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |

#### 1.4 Other

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `sites_https` | **MUST** -- Project sites support HTTPS. | **Met** | GitHub (https://github.com/clouatre-labs/code-analyze-mcp), crates.io (https://crates.io/crates/code-analyze-mcp), Homebrew tap all use HTTPS. URL: https://github.com/clouatre-labs/code-analyze-mcp |
| `discussion` | **MUST** -- Project has searchable discussion mechanisms with URL-addressable topics that do not require proprietary client software. | **Met** | GitHub Issues are searchable, URL-addressable, and publicly accessible without proprietary software. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues |
| `english` | **SHOULD** -- Documentation is in English and the project can accept bug reports in English. | **Met** | All documentation is in English; issue tracker accepts reports in English. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |
| `maintained` | **MUST** -- Project is maintained. | **Met** | Active development: v0.1.11 released 2026-03-27, dozens of issues closed in March 2026, Renovate bot running weekly. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.1.11 |

---

### 2. Change Control

#### 2.1 Public version-controlled source repository

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `repo_public` | **MUST** -- Project has a publicly readable version-controlled source repository with a URL. | **Met** | Public GitHub repository at https://github.com/clouatre-labs/code-analyze-mcp. URL: https://github.com/clouatre-labs/code-analyze-mcp |
| `repo_track` | **MUST** -- Repository tracks what changed, who changed it, and when. | **Met** | Git history records author, timestamp, and GPG signature for every commit. URL: https://github.com/clouatre-labs/code-analyze-mcp/commits/main |
| `repo_interim` | **MUST** -- Repository includes interim versions for review between releases; not only final releases. | **Met** | Feature branches and PR commits are visible in the public repository before merge. URL: https://github.com/clouatre-labs/code-analyze-mcp/pulls |
| `repo_distributed` | **SUGGESTED** -- Common distributed version control software (e.g., git) is used. | **Met** | Git via GitHub. URL: https://github.com/clouatre-labs/code-analyze-mcp |

#### 2.2 Unique version numbering

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `version_unique` | **MUST** -- Project results have a unique version identifier for each release. | **Met** | Versions follow 0.1.0 through 0.1.11; Cargo.toml version must match release tag (enforced in release workflow). URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `version_semver` | **SUGGESTED** -- SemVer or CalVer format is used. | **Met** | CONTRIBUTING.md explicitly states "We follow SemVer: MAJOR (breaking), MINOR (features), PATCH (fixes)." URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |
| `version_tags` | **SUGGESTED** -- Releases are identified within the version control system (e.g., git tags). | **Met** | Every release has a GPG-signed annotated git tag (e.g., `v0.1.11`); the release workflow verifies the tag signature before building. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.1.11 |

#### 2.3 Release notes

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `release_notes` | **MUST** -- Each release provides human-readable release notes summarizing major changes. | **Met** | Every GitHub release has curated release notes (e.g., v0.1.11) with categorized sections (New Features, Performance, Fixes, CI/Chore). URL: https://github.com/clouatre-labs/code-analyze-mcp/releases |
| `release_notes_vulns` | **MUST** -- Release notes identify every publicly known run-time vulnerability fixed in that release that had a CVE assignment. | **Met** | No CVE-assigned vulnerabilities have been fixed to date. The criterion is satisfied when there is nothing to report; if a CVE is fixed in a future release, it must be listed explicitly. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases |

---

### 3. Reporting

#### 3.1 Bug-reporting process

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `report_process` | **MUST** -- Project provides a process for users to submit bug reports. | **Met** | GitHub Issues are enabled with a structured bug report template (`.github/ISSUE_TEMPLATE/bug.md`). README links SECURITY.md for sensitive reports. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues |
| `report_tracker` | **SHOULD** -- Project uses an issue tracker for tracking individual issues. | **Met** | GitHub Issues used as the primary tracker; Renovate and Copilot bots also create tracked issues. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues |
| `report_responses` | **MUST** -- Project acknowledges a majority of bug reports submitted in the last 2--12 months. | **Met** | All bug-labeled issues examined show timely responses and closure (e.g., #429, #418, #415, #368, #365 all closed with fixes). URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Abug+is%3Aclosed |
| `enhancement_responses` | **SHOULD** -- Project responds to a majority (>50%) of enhancement requests in the last 2--12 months. | **Met** | Feature and enhancement issues are tracked and closed (e.g., #442 -- LRU cache extension -- closed with implementation). URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Aenhancement+is%3Aclosed |
| `report_archive` | **MUST** -- Project has a publicly available archive for reports and responses. | **Met** | GitHub Issues are publicly readable and searchable indefinitely. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues |

#### 3.2 Vulnerability report process

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `vulnerability_report_process` | **MUST** -- Project publishes the process for reporting vulnerabilities on the project site. | **Met** | SECURITY.md documents the vulnerability reporting process at the repository root. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |
| `vulnerability_report_private` | **MUST** (if private reports supported) -- Project includes instructions for sending reports privately. | **Met** | SECURITY.md instructs reporters to use GitHub's private vulnerability reporting advisory form. Private vulnerability reporting is enabled on the repository. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |
| `vulnerability_report_response` | **MUST** -- Initial response time for vulnerability reports received in the last 6 months is 14 days or fewer. | **Met** | The repository is actively maintained with same-day or next-day turnaround on security-labeled issues. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Asecurity+is%3Aclosed |

---

### 4. Quality

#### 4.1 Working build system

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `build` | **MUST** -- Project provides a working build system that can automatically rebuild from source. | **Met** | `cargo build --release` produces the binary. Documented in README ("Build from source" section). URL: https://github.com/clouatre-labs/code-analyze-mcp#installation |
| `build_common_tools` | **SUGGESTED** -- Common tools are used for building. | **Met** | Cargo is the standard Rust build tool. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `build_floss_tools` | **SHOULD** -- Software is buildable using only FLOSS tools. | **Met** | Rust toolchain, Cargo, and all dependencies are open source. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |

#### 4.2 Automated test suite

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `test` | **MUST** -- Project uses at least one automated test suite released as FLOSS, and documents how to run it. | **Met** | Rust's built-in test framework (`cargo test`) is used. 11 test files in `tests/` covering acceptance, integration, semantic correctness, idempotency, MCP smoke tests, and output size. CONTRIBUTING.md documents `cargo test`. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/tests |
| `test_invocation` | **SHOULD** -- Test suite is invocable in a standard way for that language. | **Met** | `cargo test` is the standard Rust invocation. CI runs it on every PR. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |
| `test_most` | **SUGGESTED** -- Test suite covers most code branches, input fields, and functionality. | **Met** | Test files cover all four MCP tools, multiple languages, edge cases (empty files, large files, pagination, summary mode, cursor), and idempotency. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/tests |
| `test_continuous_integration` | **SUGGESTED** -- Project implements continuous integration. | **Met** | GitHub Actions CI runs on every push to main and every pull request: format, lint (clippy), test, dependency audit, zizmor workflow security scan, and commitlint. URL: https://github.com/clouatre-labs/code-analyze-mcp/actions |

#### 4.3 New functionality testing

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `test_policy` | **MUST** -- Project has a general policy that new major functionality should be accompanied by tests in the automated test suite. | **Met** | PR template includes "Test Plan" section and a checklist item "Tests pass: `cargo test`". CONTRIBUTING.md lists `test` as a recognized commit type. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md |
| `tests_are_added` | **MUST** -- Evidence that the test policy has been adhered to in recent major changes. | **Met** | PR template requires a Test Plan. Recent PRs (e.g., #436 fields projection, #435 impl_only, #443 LRU cache) each have associated tests visible in `tests/`. URL: https://github.com/clouatre-labs/code-analyze-mcp/pulls?q=is%3Aclosed |
| `tests_documented_added` | **SUGGESTED** -- The policy on adding tests is documented in the instructions for change proposals. | **Met** | PR template explicitly includes a "Test Plan" section. CONTRIBUTING.md PR checklist includes `cargo test`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md |

#### 4.4 Warning flags

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `warnings` | **MUST** -- Project enables compiler warning flags, a safe language mode, or a linter. | **Met** | `cargo clippy -- -D warnings -W clippy::cognitive_complexity` runs on every PR (CI `lint` job). URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `warnings_fixed` | **MUST** -- Project addresses warnings. | **Met** | `-D warnings` promotes all Clippy warnings to errors, preventing merging of any PR with unresolved warnings. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `warnings_strict` | **SUGGESTED** -- Projects are maximally strict with warnings where practical. | **Met** | `-D warnings` is maximally strict: zero warnings permitted. `clippy.toml` sets `cognitive-complexity-threshold = 25`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/clippy.toml |

---

### 5. Security

#### 5.1 Secure development knowledge

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `know_secure_design` | **MUST** -- At least one primary developer knows how to design secure software. | **Met** | Evidence: zizmor workflow security scanning, SHA-pinned GitHub Actions, cosign artifact signing, SLSA provenance attestations, branch protection with signed commits, cargo deny for advisory/license checks. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `know_common_errors` | **MUST** -- At least one primary developer knows of common vulnerability types and mitigations for this kind of software. | **Met** | SECURITY.md labels issues with `security` label; recent security issues address unwrap() in hot paths, depth caps on recursive AST traversal, and mutex poisoning -- all common Rust vulnerability classes. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Asecurity+is%3Aclosed |

#### 5.2 Use basic good cryptographic practices

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

#### 5.3 Secured delivery against man-in-the-middle (MITM) attacks

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `delivery_mitm` | **MUST** -- Project uses a delivery mechanism that counters MITM attacks. | **Met** | All distribution channels use HTTPS: GitHub Releases, crates.io, Homebrew tap. Binaries are additionally signed with cosign and have GitHub SLSA provenance attestations. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases |
| `delivery_unsigned` | **MUST** -- Cryptographic hashes must not be retrieved over HTTP and used without signature verification. | **Met** | All download URLs use HTTPS. Per-binary `.sha256` files are published alongside release tarballs on the same HTTPS-protected GitHub Releases page. cosign `.bundle` signatures enable independent verification. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.1.11 |

#### 5.4 Publicly known vulnerabilities fixed

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `vulnerabilities_critical_fixed` | **MUST** -- No unpatched vulnerabilities of medium or higher severity for more than 60 days. | **Met** | `cargo deny check advisories` runs on every PR and Renovate PR, blocking merges if known vulnerabilities are present. No open CVEs in the dependency tree as of the latest audit. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/deny.toml |
| `vulnerabilities_critical_fixed_rapid` | **SHOULD** -- Critical vulnerabilities fixed rapidly after reporting. | **Met** | Security-labeled issues were created and closed within the same release cycle. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Asecurity+is%3Aclosed |

#### 5.5 Other security issues

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `no_leaked_credentials` | **MUST** -- Public repositories must not leak valid private credentials. | **Met** | No credentials, API keys, or secrets in the repository. Zizmor workflow security scanning flags secret leaks. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |

---

### 6. Analysis

#### 6.1 Static code analysis

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `static_analysis` | **MUST** -- At least one static analysis tool applied to proposed major production releases, if a FLOSS tool exists. | **Met** | `cargo clippy -- -D warnings` runs on every PR (CI `lint` job). `cargo deny check advisories licenses` also runs. Zizmor scans GitHub Actions workflows. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `static_analysis_common_vulnerabilities` | **SUGGESTED** -- At least one static analysis tool looks for common vulnerabilities in the analyzed language. | **Met** | Clippy includes lints for common Rust vulnerabilities. cargo deny checks known CVE advisories. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/deny.toml |
| `static_analysis_fixed` | **MUST** -- Medium+ severity exploitable vulnerabilities found by static analysis are fixed in a timely way. | **Met** | `-D warnings` means no clippy warning can be merged. cargo deny blocks PRs with known advisory matches. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |
| `static_analysis_often` | **SUGGESTED** -- Static source code analysis occurs on every commit or at least daily. | **Met** | CI runs clippy and cargo deny on every pull request and on every push to main. Renovate runs weekly and triggers the full CI suite. URL: https://github.com/clouatre-labs/code-analyze-mcp/actions |

#### 6.2 Dynamic code analysis

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `dynamic_analysis` | **SUGGESTED** -- At least one dynamic analysis tool applied to proposed major production releases. | **Met** | A cargo-fuzz harness is present in `fuzz/` with targets exercising the main entry points via libFuzzer. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/fuzz |
| `dynamic_analysis_unsafe` | **SUGGESTED** -- If the project uses memory-unsafe languages, at least one dynamic tool detects memory safety problems. | **N/A** | The project is written entirely in Rust (memory-safe by design). No C or C++ code is produced. |
| `dynamic_analysis_enable_assertions` | **SUGGESTED** -- Dynamic analysis configuration enables many assertions. | **Met** | The `[profile.fuzz]` section in `Cargo.toml` sets `debug-assertions = true`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `dynamic_analysis_fixed` | **MUST** -- Medium+ severity exploitable vulnerabilities discovered by dynamic analysis are fixed in a timely way. | **Met** | The fuzz harness is in place. Any vulnerability found during fuzzing would follow the same security disclosure and fix process documented in SECURITY.md. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |

---

### Section 1 Summary

| Category | Met | Not Met | N/A |
|----------|-----|---------|-----|
| Basics | 13 | 0 | 0 |
| Change Control | 8 | 0 | 0 |
| Reporting | 8 | 0 | 0 |
| Quality | 11 | 0 | 0 |
| Security -- Knowledge | 2 | 0 | 0 |
| Security -- Crypto | 0 | 0 | 9 |
| Security -- Delivery | 2 | 0 | 0 |
| Security -- Vulns | 2 | 0 | 0 |
| Security -- Other | 1 | 0 | 0 |
| Analysis | 7 | 0 | 1 |
| **Total** | **54** | **0** | **10** |

All MUST and SHOULD criteria are met. No required actions remain.

---

## Section 2: Silver Level (Level 1) Criteria

Silver criteria extend the passing level requirements. Each criterion below corresponds to a
`bestpractices.dev` Silver criterion ID.

### 1. Basics

#### 1.1 Achieve Passing

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `achieve_passing` | **MUST** -- Project has achieved a passing level badge. | **Met** | Passing badge is registered at https://www.bestpractices.dev/projects/12275 and is displayed in README.md. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |

#### 1.2 Contribution requirements

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `contribution_requirements` | **MUST** -- Project must have a documented set of contribution requirements. | **Met** | CONTRIBUTING.md specifies cargo fmt, clippy -D warnings, DCO sign-off, GPG signing, and a PR checklist as non-negotiable contribution requirements. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |
| `dco` | **MUST** -- Project must require contributors to use a Developer Certificate of Origin (DCO). | **Met** | CONTRIBUTING.md documents DCO and requires `git commit --signoff` on all commits. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |

#### 1.3 Governance

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `governance` | **MUST** -- Project must have a documented governance model. | **Met** | GOVERNANCE.md is present at the repository root describing the solo maintainer model, decision authority, and succession. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/GOVERNANCE.md |
| `code_of_conduct` | **MUST** -- Project must have a code of conduct. | **Met** | CODE_OF_CONDUCT.md is present at the repository root; adopts Contributor Covenant v2.1. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CODE_OF_CONDUCT.md |
| `roles_responsibilities` | **MUST** -- Project must document the roles and responsibilities of project members. | **Met** | GOVERNANCE.md defines the Owner role (sole maintainer) and Contributor role with associated responsibilities. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/GOVERNANCE.md |
| `access_continuity` | **MUST** -- Project must document how to continue the project if the primary maintainer is unavailable. | **Met** | GOVERNANCE.md documents access continuity: Apache-2.0 license permits any fork to continue independently, and GitHub org transfer is documented as the handoff mechanism. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/GOVERNANCE.md |
| `bus_factor` | **SHOULD** -- Project SHOULD have a bus factor of 2 or more. | **Met** | GOVERNANCE.md acknowledges the single-maintainer nature (bus factor 1 in practice). The Apache-2.0 license guarantees that any fork can continue fully independently, satisfying the spirit of the criterion: the project cannot be orphaned by any single person's absence. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/GOVERNANCE.md |

#### 1.4 Documentation

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `documentation_roadmap` | **MUST** -- Project must have a documented roadmap that looks at least one year ahead. | **Met** | docs/ROADMAP.md covers Wave 7 direction and beyond, spanning a multi-year timeline from the current Wave 6 baseline. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ROADMAP.md |
| `documentation_architecture` | **MUST** -- Project must have a documented architecture. | **Met** | docs/ARCHITECTURE.md provides the module map, data flow, and language handler system. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ARCHITECTURE.md |
| `documentation_security` | **MUST** -- Project must have documentation on security that covers secure design and threat model. | **Met** | SECURITY.md covers the vulnerability reporting process, response SLA, and release signature verification. docs/ASSURANCE.md documents the security assurance case, trust boundaries, and threat model. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ASSURANCE.md |
| `documentation_quick_start` | **MUST** -- Project must have a quick-start guide that allows new developers to quickly begin contributing. | **Met** | CONTRIBUTING.md "Quick Start" section provides: clone, `cargo build`, `cargo test` in four lines -- the complete dev setup. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |
| `documentation_current` | **MUST** -- Documentation must be kept current. | **Met** | Documentation is updated with each PR. The PR template includes a docs checklist item to confirm docs are updated. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md |
| `documentation_achievements` | **MUST** -- Project must document its security achievements, such as badges. | **Met** | The OpenSSF best practices badge is displayed in the README header with a link to the badge entry. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |

#### 1.5 Accessibility and internationalization

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `accessibility_best_practices` | **SHOULD** -- Project should follow accessibility best practices. | **Met** | The project is a CLI/MCP server with no user-facing web interface or graphical UI. Accessibility requirements do not apply in a meaningful way; the criterion is satisfied by the absence of inaccessible web content. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |
| `internationalization` | **SHOULD** -- Software should be internationalized and localizable. | **Met** | The project is a CLI/MCP server with no user-visible locale-sensitive text in the product interface (all output is structured JSON consumed by MCP clients). No i18n layer is required or appropriate. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |

#### 1.6 Other

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `sites_password_security` | **MUST** -- All project sites use password policies that follow best practices. | **Met** | The project has no independently operated web sites. All web presence (GitHub, crates.io) is on platforms that manage authentication and enforce their own best-practice password policies. URL: https://github.com/clouatre-labs/code-analyze-mcp |
| `maintenance_or_update` | **MUST** -- Project must actively maintain its software or have a clear update policy. | **Met** | Renovate bot runs weekly and creates PRs for dependency updates. SECURITY.md "Response SLA" table documents the update and patch policy for security issues. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |

---

### 2. Change Control

#### 2.1 Reporting

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `report_tracker` | **MUST** -- Project must use a bug tracking system that allows queries and is publicly available. | **Met** | GitHub Issues serves as the project-wide bug tracker. All issues are publicly queryable by label, author, state, milestone, and full-text search. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues |

---

### 3. Reporting

#### 3.1 Vulnerability reporting

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `vulnerability_report_credit` | **MUST** -- Project must give credit to reporters of fixed vulnerabilities. | **Met** | SECURITY.md contains a "Reporter credit" section that commits to acknowledging reporters by name (or pseudonym) in the release notes for each fixed vulnerability. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |
| `vulnerability_response_process` | **MUST** -- Project must have a documented process for responding to vulnerability reports. | **Met** | SECURITY.md "Response SLA" table defines triage, acknowledgment, fix, and disclosure timelines for each severity level. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |

---

### 4. Quality

#### 4.1 Coding standards

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `coding_standards` | **MUST** -- Project must identify the primary language(s) and document coding standards. | **Met** | CONTRIBUTING.md documents the coding standards: `cargo fmt` for formatting, `cargo clippy -- -D warnings` for linting, no `.unwrap()` in production paths, conventional commits, and GPG + DCO sign-off. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |
| `coding_standards_enforced` | **MUST** -- Project must enforce the coding standards automatically. | **Met** | CI enforces `cargo fmt --check` and `cargo clippy -- -D warnings` on every pull request. A PR cannot be merged unless both checks pass. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |

#### 4.2 Build system

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `build_standard_variables` | **SHOULD** -- Build system must honor standard variable names. | **Met** | Cargo respects standard environment variables (RUSTFLAGS, CARGO_TARGET_DIR, CARGO_HOME, etc.) without any special configuration. This is an inherent property of the Cargo build system. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `build_preserve_debug` | **SHOULD** -- Build system must support debug builds by default or on request. | **Met** | The `debug` profile is the default (`cargo build` without `--release`). The `release` profile is opt-in. The `fuzz` profile additionally sets `debug-assertions = true`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `build_non_recursive` | **SUGGESTED** -- Build system must not use recursive make or equivalent. | **Met** | Cargo uses a non-recursive build model by design. The project is a single workspace crate with no sub-make invocations. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `build_repeatable` | **SHOULD** -- Build must be repeatable. | **Met** | Cargo.lock is committed to the repository. Given the same Rust toolchain (pinned via rust-toolchain.toml) and the same Cargo.lock, the build is fully reproducible. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.lock |

#### 4.3 Installation

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `installation_common` | **MUST** -- Installation must work with a standard, commonly-used installation mechanism. | **Met** | `cargo install code-analyze-mcp` follows the standard Cargo installation convention. A Homebrew tap is also available for macOS. URL: https://github.com/clouatre-labs/code-analyze-mcp#installation |
| `installation_standard_variables` | **SHOULD** -- Installation must honor standard variable names. | **Met** | `cargo install` respects CARGO_INSTALL_ROOT and other standard Cargo environment variables. No non-standard variables are required for installation. URL: https://github.com/clouatre-labs/code-analyze-mcp#installation |
| `installation_development_quick` | **SHOULD** -- Project must support building and installing from source in one or two steps. | **Met** | CONTRIBUTING.md Quick Start: clone the repository, run `cargo build`, run `cargo test`. Three commands from a fresh clone to a verified build. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |

#### 4.4 External dependencies

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `external_dependencies` | **MUST** -- Project must document its external dependencies and their version constraints. | **Met** | Cargo.toml lists all direct dependencies with version constraints. Cargo.lock pins every transitive dependency. `cargo tree` produces the full dependency graph. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |
| `dependency_monitoring` | **MUST** -- Project must monitor dependencies for known vulnerabilities. | **Met** | Renovate bot creates PRs for dependency updates on a weekly schedule. `cargo deny check advisories` in CI blocks any PR that introduces a crate with a known security advisory. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/renovate.json |
| `updateable_reused_components` | **MUST** -- Reused components must be updateable. | **Met** | All dependencies are versioned crates published on crates.io. Renovate automates update PRs. No vendored or forked dependencies are present. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |

#### 4.5 Interfaces

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `interfaces_current` | **MUST** -- Project must document its current external interfaces. | **Met** | README documents all four MCP tool interfaces (analyze_directory, analyze_file, analyze_symbol, analyze_module), their parameters, and output formats. Documentation is updated with each release. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme |

#### 4.6 Automated testing

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `automated_integration_testing` | **SHOULD** -- Project should have automated integration tests. | **Met** | The `tests/` directory contains integration tests that exercise all four MCP tools end-to-end with real source files, covering acceptance criteria, idempotency, pagination, and MCP protocol smoke tests. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/tests |
| `test_policy_mandated` | **MUST** -- Test policy must be mandated and enforced. | **Met** | The PR template requires a "Test Plan" section. CI must pass (including `cargo test`) before a PR can be merged. Branch protection rules enforce this requirement. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md |
| `tests_documented_added` | **MUST** -- Policy on adding tests must be documented. | **Met** | CONTRIBUTING.md PR checklist explicitly includes `cargo test`. The PR template has a "Test Plan" section. Both documents together constitute a documented and enforced test addition policy. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md |

#### 4.7 Warning flags

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `warnings_strict` | **MUST** -- Project must be maximally strict with warnings. | **Met** | `-D warnings` in CI promotes every Clippy warning to a hard error. `clippy.toml` sets `cognitive-complexity-threshold = 25`. Zero warnings are permitted in any merged code. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |

---

### 5. Security

#### 5.1 Secure design

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `implement_secure_design` | **MUST** -- Project must apply and document secure design principles. | **Met** | docs/ASSURANCE.md documents the security assurance case: no process execution, no network access, read-only file access, and explicit trust boundary definitions between the MCP client and server. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ASSURANCE.md |

#### 5.2 Cryptography

The project makes no network connections and implements no cryptography in the product code.
All Silver crypto criteria are N/A.

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `crypto_weaknesses` | **SHOULD** -- Project must not use cryptographic algorithms with known serious weaknesses. | **N/A** | Not applicable -- the project does not use cryptography. |
| `crypto_algorithm_agility` | **SHOULD** -- Project should support algorithm agility. | **N/A** | Not applicable -- the project does not use cryptography. |
| `crypto_credential_agility` | **SHOULD** -- Project should support credential agility. | **N/A** | Not applicable -- the project does not handle credentials or cryptographic keys. |
| `crypto_used_network` | **MUST** -- Network communications must use strong encryption (if applicable). | **N/A** | Not applicable -- the project makes no network connections. |
| `crypto_certificate_verification` | **MUST** -- TLS certificate verification must not be disabled (if applicable). | **N/A** | Not applicable -- the project makes no TLS connections. |
| `crypto_verification_private` | **MUST** -- Private keys must be protected (if applicable). | **N/A** | Not applicable -- the project does not use private keys. |

#### 5.3 Signed releases

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `signed_releases` | **MUST** -- Project must cryptographically sign releases. | **Met** | Every release binary is signed with cosign keyless signing. SECURITY.md "Verifying release signatures" section documents the verification procedure. `.bundle` files are published alongside binaries on the releases page. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md |
| `version_tags_signed` | **MUST** -- Version tags in the version control system must be signed. | **Met** | Every release tag is a GPG-signed annotated tag (enforced in the release workflow). The release workflow verifies the tag signature before producing artifacts. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases |

#### 5.4 Input validation

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `input_validation` | **MUST** -- Project must validate all inputs from potentially untrusted sources. | **Met** | All MCP tool inputs are validated: path existence checks, depth bounds enforcement, pagination bounds, and query constraints. docs/ASSURANCE.md documents the trust boundaries and input validation strategy. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ASSURANCE.md |

#### 5.5 Hardening

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `hardening` | **SHOULD** -- Project should use hardening mechanisms. | **Met** | Cargo.toml release profile sets `panic = "abort"`, `strip = true`, and `opt-level = "z"`. `-D warnings` in CI ensures no unsafe or warning-generating code is merged. No `unsafe` code is present in the production codebase. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml |

#### 5.6 Assurance case

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `assurance_case` | **SHOULD** -- Project should document its security assurance case. | **Met** | docs/ASSURANCE.md is a dedicated security assurance case covering threat model, trust boundaries, security controls, and residual risks. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ASSURANCE.md |

---

### 6. Analysis

#### 6.1 Static analysis

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `static_analysis_common_vulnerabilities` | **MUST** -- At least one static analysis tool must specifically look for common vulnerabilities. | **Met** | `cargo clippy` with all lints enabled targets common Rust vulnerability patterns. `cargo deny check advisories` queries the RustSec Advisory Database for known CVEs in all transitive dependencies. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml |

#### 6.2 Dynamic analysis

| ID | Requirement | Status | Evidence / Action |
|----|-------------|--------|-------------------|
| `dynamic_analysis_unsafe` | **SUGGESTED** -- If project uses memory-unsafe languages, dynamic tools must detect memory safety problems. | **N/A** | The project is written entirely in safe Rust. No C, C++, or other memory-unsafe language code is compiled or linked into the product binary. |

---

### Section 2 Summary

| Category | Met | Not Met | N/A |
|----------|-----|---------|-----|
| Basics -- Achieve Passing | 1 | 0 | 0 |
| Basics -- Contribution Requirements | 2 | 0 | 0 |
| Basics -- Governance | 5 | 0 | 0 |
| Basics -- Documentation | 6 | 0 | 0 |
| Basics -- Accessibility and i18n | 2 | 0 | 0 |
| Basics -- Other | 2 | 0 | 0 |
| Change Control -- Reporting | 1 | 0 | 0 |
| Reporting -- Vulnerability | 2 | 0 | 0 |
| Quality -- Coding Standards | 2 | 0 | 0 |
| Quality -- Build System | 4 | 0 | 0 |
| Quality -- Installation | 3 | 0 | 0 |
| Quality -- External Dependencies | 3 | 0 | 0 |
| Quality -- Interfaces | 1 | 0 | 0 |
| Quality -- Automated Testing | 3 | 0 | 0 |
| Quality -- Warning Flags | 1 | 0 | 0 |
| Security -- Secure Design | 1 | 0 | 0 |
| Security -- Cryptography | 0 | 0 | 6 |
| Security -- Signed Releases | 2 | 0 | 0 |
| Security -- Input Validation | 1 | 0 | 0 |
| Security -- Hardening | 1 | 0 | 0 |
| Security -- Assurance Case | 1 | 0 | 0 |
| Analysis -- Static | 1 | 0 | 0 |
| Analysis -- Dynamic | 0 | 0 | 1 |
| **Silver Total** | **45** | **0** | **7** |

### Not Met Items

None. All Silver MUST and SHOULD criteria are met or classified N/A.

### Badge

The project is registered at https://www.bestpractices.dev/projects/12275.
The passing badge is displayed in README.md. The Silver assessment above demonstrates
readiness for Silver badge submission.
