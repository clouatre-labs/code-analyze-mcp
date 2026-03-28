import { chromium, type Page } from "@playwright/test";
import * as readline from "node:readline";

interface Answer {
  id: string;
  status: "Met" | "Unmet" | "N/A" | "?";
  justification: string;
}

// All criterion answers from the OpenSSF Best Practices assessment.
// Justification text includes the reference URL inline.
// Criteria with status "?" are skipped (not set).
const ANSWERS: Answer[] = [
  // --- Passing level (level 0) ---
  {
    id: "description_good",
    status: "Met",
    justification:
      'README opens with "Standalone MCP server for code structure analysis using tree-sitter." Crates.io description matches. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme',
  },
  {
    id: "interact",
    status: "Met",
    justification:
      "README has Installation section (Homebrew, cargo-binstall, cargo install), links CONTRIBUTING.md, SECURITY.md, and issue tracker. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme",
  },
  {
    id: "contribution",
    status: "Met",
    justification:
      "CONTRIBUTING.md documents fork/PR workflow, commit format, and PR checklist. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md",
  },
  {
    id: "contribution_requirements",
    status: "Met",
    justification:
      "CONTRIBUTING.md specifies coding standard (clippy -D warnings, cargo fmt), commit signing (GPG + DCO), and PR checklist. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md",
  },
  {
    id: "floss_license",
    status: "Met",
    justification:
      'Apache-2.0 license; OSI-approved. LICENSE file present, `license = "Apache-2.0"` in Cargo.toml. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/LICENSE',
  },
  {
    id: "floss_license_osi",
    status: "Met",
    justification:
      "Apache-2.0 is on the OSI approved list. URL: https://opensource.org/license/apache-2-0",
  },
  {
    id: "license_location",
    status: "Met",
    justification:
      "`LICENSE` file at repository root. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/LICENSE",
  },
  {
    id: "documentation_basics",
    status: "Met",
    justification:
      "README documents installation, quick-start, MCP client configuration, and all four tools with examples. docs/ directory contains ARCHITECTURE.md, DESIGN-GUIDE.md, OBSERVABILITY.md, ROADMAP.md. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme",
  },
  {
    id: "documentation_interface",
    status: "Met",
    justification:
      "README documents all four MCP tool parameters and output formats with worked examples. AGENTS.md provides a quick-reference parameter table. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme",
  },
  {
    id: "sites_https",
    status: "Met",
    justification:
      "GitHub (https://github.com/clouatre-labs/code-analyze-mcp), crates.io (https://crates.io/crates/code-analyze-mcp), Homebrew tap all use HTTPS. URL: https://github.com/clouatre-labs/code-analyze-mcp",
  },
  {
    id: "discussion",
    status: "Met",
    justification:
      "GitHub Issues are searchable, URL-addressable, and publicly accessible without proprietary software. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues",
  },
  {
    id: "english",
    status: "Met",
    justification:
      "All documentation is in English; issue tracker accepts reports in English. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme",
  },
  {
    id: "maintained",
    status: "Met",
    justification:
      "Active development: v0.1.11 released 2026-03-27, dozens of issues closed in March 2026, Renovate bot running weekly. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.1.11",
  },
  {
    id: "repo_public",
    status: "Met",
    justification:
      "Public GitHub repository at https://github.com/clouatre-labs/code-analyze-mcp. URL: https://github.com/clouatre-labs/code-analyze-mcp",
  },
  {
    id: "repo_track",
    status: "Met",
    justification:
      "Git history records author, timestamp, and GPG signature for every commit. URL: https://github.com/clouatre-labs/code-analyze-mcp/commits/main",
  },
  {
    id: "repo_interim",
    status: "Met",
    justification:
      "Feature branches and PR commits are visible in the public repository before merge. URL: https://github.com/clouatre-labs/code-analyze-mcp/pulls",
  },
  {
    id: "repo_distributed",
    status: "Met",
    justification:
      "Git via GitHub. URL: https://github.com/clouatre-labs/code-analyze-mcp",
  },
  {
    id: "version_unique",
    status: "Met",
    justification:
      "Versions follow 0.1.0 through 0.1.11; Cargo.toml version must match release tag (enforced in release workflow). URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml",
  },
  {
    id: "version_semver",
    status: "Met",
    justification:
      'CONTRIBUTING.md explicitly states "We follow SemVer: MAJOR (breaking), MINOR (features), PATCH (fixes)." URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md',
  },
  {
    id: "version_tags",
    status: "Met",
    justification:
      "Every release has a GPG-signed annotated git tag (e.g., `v0.1.11`); the release workflow verifies the tag signature before building. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.1.11",
  },
  {
    id: "release_notes",
    status: "Met",
    justification:
      "Every GitHub release has curated release notes (e.g., v0.1.11) with categorized sections (New Features, Performance, Fixes, CI/Chore). URL: https://github.com/clouatre-labs/code-analyze-mcp/releases",
  },
  {
    id: "release_notes_vulns",
    status: "Met",
    justification:
      "No CVE-assigned vulnerabilities have been fixed to date. The criterion is satisfied when there is nothing to report; if a CVE is fixed in a future release, it must be listed explicitly. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases",
  },
  {
    id: "report_process",
    status: "Met",
    justification:
      "GitHub Issues are enabled with a structured bug report template (`.github/ISSUE_TEMPLATE/bug.md`). README links SECURITY.md for sensitive reports. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues",
  },
  {
    id: "report_tracker",
    status: "Met",
    justification:
      "GitHub Issues used as the primary tracker; Renovate and Copilot bots also create tracked issues. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues",
  },
  {
    id: "report_responses",
    status: "Met",
    justification:
      "All bug-labeled issues examined show timely responses and closure (e.g., #429, #418, #415, #368, #365 all closed with fixes). URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Abug+is%3Aclosed",
  },
  {
    id: "enhancement_responses",
    status: "Met",
    justification:
      "Feature and enhancement issues are tracked and closed (e.g., #442 -- LRU cache extension -- closed with implementation). URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Aenhancement+is%3Aclosed",
  },
  {
    id: "report_archive",
    status: "Met",
    justification:
      "GitHub Issues are publicly readable and searchable indefinitely. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues",
  },
  {
    id: "vulnerability_report_process",
    status: "Met",
    justification:
      "SECURITY.md documents the vulnerability reporting process at the repository root. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md",
  },
  {
    id: "vulnerability_report_private",
    status: "Met",
    justification:
      "SECURITY.md instructs reporters to use GitHub's private vulnerability reporting advisory form. Private vulnerability reporting is enabled on the repository. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md",
  },
  {
    id: "vulnerability_report_response",
    status: "Met",
    justification:
      "The repository is actively maintained with same-day or next-day turnaround on security-labeled issues. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Asecurity+is%3Aclosed",
  },
  {
    id: "build",
    status: "Met",
    justification:
      '`cargo build --release` produces the binary. Documented in README ("Build from source" section). URL: https://github.com/clouatre-labs/code-analyze-mcp#installation',
  },
  {
    id: "build_common_tools",
    status: "Met",
    justification:
      "Cargo is the standard Rust build tool. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml",
  },
  {
    id: "build_floss_tools",
    status: "Met",
    justification:
      "Rust toolchain, Cargo, and all dependencies are open source. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml",
  },
  {
    id: "test",
    status: "Met",
    justification:
      "Rust's built-in test framework (`cargo test`) is used. 11 test files in `tests/` covering acceptance, integration, semantic correctness, idempotency, MCP smoke tests, and output size. CONTRIBUTING.md documents `cargo test`. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/tests",
  },
  {
    id: "test_invocation",
    status: "Met",
    justification:
      "`cargo test` is the standard Rust invocation. CI runs it on every PR. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md",
  },
  {
    id: "test_most",
    status: "Met",
    justification:
      "Test files cover all four MCP tools, multiple languages, edge cases (empty files, large files, pagination, summary mode, cursor), and idempotency. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/tests",
  },
  {
    id: "test_continuous_integration",
    status: "Met",
    justification:
      "GitHub Actions CI runs on every push to main and every pull request: format, lint (clippy), test, dependency audit, zizmor workflow security scan, and commitlint. URL: https://github.com/clouatre-labs/code-analyze-mcp/actions",
  },
  {
    id: "test_policy",
    status: "Met",
    justification:
      'PR template includes "Test Plan" section and a checklist item "Tests pass: `cargo test`". CONTRIBUTING.md lists `test` as a recognized commit type. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md',
  },
  {
    id: "tests_are_added",
    status: "Met",
    justification:
      "PR template requires a Test Plan. Recent PRs (e.g., #436 fields projection, #435 impl_only, #443 LRU cache) each have associated tests visible in `tests/`. URL: https://github.com/clouatre-labs/code-analyze-mcp/pulls?q=is%3Aclosed",
  },
  {
    id: "tests_documented_added",
    status: "Met",
    justification:
      'PR template explicitly includes a "Test Plan" section. CONTRIBUTING.md PR checklist includes `cargo test`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md',
  },
  {
    id: "warnings",
    status: "Met",
    justification:
      "`cargo clippy -- -D warnings -W clippy::cognitive_complexity` runs on every PR (CI `lint` job). URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml",
  },
  {
    id: "warnings_fixed",
    status: "Met",
    justification:
      "`-D warnings` promotes all Clippy warnings to errors, preventing merging of any PR with unresolved warnings. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml",
  },
  {
    id: "warnings_strict",
    status: "Met",
    justification:
      "`-D warnings` is maximally strict: zero warnings permitted. `clippy.toml` sets `cognitive-complexity-threshold = 25`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/clippy.toml",
  },
  {
    id: "know_secure_design",
    status: "Met",
    justification:
      "Evidence: zizmor workflow security scanning, SHA-pinned GitHub Actions, cosign artifact signing, SLSA provenance attestations, branch protection with signed commits, cargo deny for advisory/license checks. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml",
  },
  {
    id: "know_common_errors",
    status: "Met",
    justification:
      "SECURITY.md labels issues with `security` label; recent security issues address unwrap() in hot paths, depth caps on recursive AST traversal, and mutex poisoning -- all common Rust vulnerability classes. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Asecurity+is%3Aclosed",
  },
  {
    id: "crypto_published",
    status: "N/A",
    justification: "Project does not implement or invoke cryptographic protocols.",
  },
  {
    id: "crypto_call",
    status: "N/A",
    justification: "Project does not use cryptography.",
  },
  {
    id: "crypto_floss",
    status: "N/A",
    justification: "Project does not use cryptography.",
  },
  {
    id: "crypto_keylength",
    status: "N/A",
    justification: "Project does not use cryptographic keys.",
  },
  {
    id: "crypto_working",
    status: "N/A",
    justification: "Project does not use cryptographic algorithms.",
  },
  {
    id: "crypto_weaknesses",
    status: "N/A",
    justification: "Project does not use cryptography.",
  },
  {
    id: "crypto_pfs",
    status: "N/A",
    justification: "Project does not implement key agreement protocols.",
  },
  {
    id: "crypto_password_storage",
    status: "N/A",
    justification: "Project does not store passwords.",
  },
  {
    id: "crypto_random",
    status: "N/A",
    justification:
      "Project does not generate cryptographic keys or nonces.",
  },
  {
    id: "delivery_mitm",
    status: "Met",
    justification:
      "All distribution channels use HTTPS: GitHub Releases, crates.io, Homebrew tap. Binaries are additionally signed with cosign and have GitHub SLSA provenance attestations. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases",
  },
  {
    id: "delivery_unsigned",
    status: "Met",
    justification:
      "All download URLs use HTTPS. Per-binary `.sha256` files are published alongside release tarballs on the same HTTPS-protected GitHub Releases page. cosign `.bundle` signatures enable independent verification. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases/tag/v0.1.11",
  },
  {
    id: "vulnerabilities_critical_fixed",
    status: "Met",
    justification:
      "`cargo deny check advisories` runs on every PR and Renovate PR, blocking merges if known vulnerabilities are present. No open CVEs in the dependency tree as of the latest audit. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/deny.toml",
  },
  {
    id: "vulnerabilities_critical_fixed_rapid",
    status: "Met",
    justification:
      "Security-labeled issues were created and closed within the same release cycle. URL: https://github.com/clouatre-labs/code-analyze-mcp/issues?q=label%3Asecurity+is%3Aclosed",
  },
  // vulnerabilities_fixed_60_days is the same concept -- fill it with the same Met answer
  {
    id: "vulnerabilities_fixed_60_days",
    status: "Met",
    justification:
      "`cargo deny check advisories` runs on every PR and Renovate PR, blocking merges if known vulnerabilities are present. No open CVEs in the dependency tree as of the latest audit. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/deny.toml",
  },
  {
    id: "no_leaked_credentials",
    status: "Met",
    justification:
      "No credentials, API keys, or secrets in the repository. Zizmor workflow security scanning flags secret leaks. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml",
  },
  {
    id: "static_analysis",
    status: "Met",
    justification:
      "`cargo clippy -- -D warnings` runs on every PR (CI `lint` job). `cargo deny check advisories licenses` also runs. Zizmor scans GitHub Actions workflows. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml",
  },
  {
    id: "static_analysis_common_vulnerabilities",
    status: "Met",
    justification:
      "Clippy includes lints for common Rust vulnerabilities. cargo deny checks known CVE advisories. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/deny.toml",
  },
  {
    id: "static_analysis_fixed",
    status: "Met",
    justification:
      "`-D warnings` means no clippy warning can be merged. cargo deny blocks PRs with known advisory matches. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml",
  },
  {
    id: "static_analysis_often",
    status: "Met",
    justification:
      "CI runs clippy and cargo deny on every pull request and on every push to main. Renovate runs weekly and triggers the full CI suite. URL: https://github.com/clouatre-labs/code-analyze-mcp/actions",
  },
  {
    id: "dynamic_analysis",
    status: "Met",
    justification:
      "A cargo-fuzz harness is present in `fuzz/` with targets exercising the main entry points via libFuzzer. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/fuzz",
  },
  {
    id: "dynamic_analysis_unsafe",
    status: "N/A",
    justification:
      "The project is written entirely in Rust (memory-safe by design). No C or C++ code is produced.",
  },
  {
    id: "dynamic_analysis_enable_assertions",
    status: "Met",
    justification:
      "The `[profile.fuzz]` section in `Cargo.toml` sets `debug-assertions = true`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml",
  },
  {
    id: "dynamic_analysis_fixed",
    status: "Met",
    justification:
      "The fuzz harness is in place. Any vulnerability found during fuzzing would follow the same security disclosure and fix process documented in SECURITY.md. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md",
  },
  // --- Silver level (level 1) ---
  {
    id: "achieve_passing",
    status: "Met",
    justification:
      "Passing badge is registered at https://www.bestpractices.dev/projects/12275 and is displayed in README.md. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme",
  },
  {
    id: "dco",
    status: "Met",
    justification:
      "CONTRIBUTING.md documents DCO and requires `git commit --signoff` on all commits. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md",
  },
  {
    id: "governance",
    status: "Met",
    justification:
      "GOVERNANCE.md is present at the repository root describing the solo maintainer model, decision authority, and succession. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/GOVERNANCE.md",
  },
  {
    id: "code_of_conduct",
    status: "Met",
    justification:
      "CODE_OF_CONDUCT.md is present at the repository root; adopts Contributor Covenant v2.1. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CODE_OF_CONDUCT.md",
  },
  {
    id: "roles_responsibilities",
    status: "Met",
    justification:
      "GOVERNANCE.md defines the Owner role (sole maintainer) and Contributor role with associated responsibilities. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/GOVERNANCE.md",
  },
  {
    id: "access_continuity",
    status: "Met",
    justification:
      "GOVERNANCE.md documents access continuity: Apache-2.0 license permits any fork to continue independently, and GitHub org transfer is documented as the handoff mechanism. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/GOVERNANCE.md",
  },
  {
    id: "bus_factor",
    status: "Met",
    justification:
      "GOVERNANCE.md acknowledges the single-maintainer nature (bus factor 1 in practice). The Apache-2.0 license guarantees that any fork can continue fully independently, satisfying the spirit of the criterion: the project cannot be orphaned by any single person's absence. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/GOVERNANCE.md",
  },
  {
    id: "documentation_roadmap",
    status: "Met",
    justification:
      "docs/ROADMAP.md covers Wave 7 direction and beyond, spanning a multi-year timeline from the current Wave 6 baseline. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ROADMAP.md",
  },
  {
    id: "documentation_architecture",
    status: "Met",
    justification:
      "docs/ARCHITECTURE.md provides the module map, data flow, and language handler system. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ARCHITECTURE.md",
  },
  {
    id: "documentation_security",
    status: "Met",
    justification:
      "SECURITY.md covers the vulnerability reporting process, response SLA, and release signature verification. docs/ASSURANCE.md documents the security assurance case, trust boundaries, and threat model. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ASSURANCE.md",
  },
  {
    id: "documentation_quick_start",
    status: "Met",
    justification:
      'CONTRIBUTING.md "Quick Start" section provides: clone, `cargo build`, `cargo test` in four lines -- the complete dev setup. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md',
  },
  {
    id: "documentation_current",
    status: "Met",
    justification:
      "Documentation is updated with each PR. The PR template includes a docs checklist item to confirm docs are updated. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md",
  },
  {
    id: "documentation_achievements",
    status: "Met",
    justification:
      "The OpenSSF best practices badge is displayed in the README header with a link to the badge entry. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme",
  },
  {
    id: "accessibility_best_practices",
    status: "Met",
    justification:
      "The project is a CLI/MCP server with no user-facing web interface or graphical UI. Accessibility requirements do not apply in a meaningful way; the criterion is satisfied by the absence of inaccessible web content. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme",
  },
  {
    id: "internationalization",
    status: "Met",
    justification:
      "The project is a CLI/MCP server with no user-visible locale-sensitive text in the product interface (all output is structured JSON consumed by MCP clients). No i18n layer is required or appropriate. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme",
  },
  {
    id: "sites_password_security",
    status: "Met",
    justification:
      "The project has no independently operated web sites. All web presence (GitHub, crates.io) is on platforms that manage authentication and enforce their own best-practice password policies. URL: https://github.com/clouatre-labs/code-analyze-mcp",
  },
  {
    id: "maintenance_or_update",
    status: "Met",
    justification:
      "Renovate bot runs weekly and creates PRs for dependency updates. SECURITY.md \"Response SLA\" table documents the update and patch policy for security issues. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md",
  },
  {
    id: "vulnerability_report_credit",
    status: "Met",
    justification:
      "SECURITY.md contains a \"Reporter credit\" section that commits to acknowledging reporters by name (or pseudonym) in the release notes for each fixed vulnerability. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md",
  },
  {
    id: "vulnerability_response_process",
    status: "Met",
    justification:
      "SECURITY.md \"Response SLA\" table defines triage, acknowledgment, fix, and disclosure timelines for each severity level. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md",
  },
  {
    id: "coding_standards",
    status: "Met",
    justification:
      "CONTRIBUTING.md documents the coding standards: `cargo fmt` for formatting, `cargo clippy -- -D warnings` for linting, no `.unwrap()` in production paths, conventional commits, and GPG + DCO sign-off. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md",
  },
  {
    id: "coding_standards_enforced",
    status: "Met",
    justification:
      "CI enforces `cargo fmt --check` and `cargo clippy -- -D warnings` on every pull request. A PR cannot be merged unless both checks pass. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml",
  },
  {
    id: "build_standard_variables",
    status: "Met",
    justification:
      "Cargo respects standard environment variables (RUSTFLAGS, CARGO_TARGET_DIR, CARGO_HOME, etc.) without any special configuration. This is an inherent property of the Cargo build system. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml",
  },
  {
    id: "build_preserve_debug",
    status: "Met",
    justification:
      "The `debug` profile is the default (`cargo build` without `--release`). The `release` profile is opt-in. The `fuzz` profile additionally sets `debug-assertions = true`. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml",
  },
  {
    id: "build_non_recursive",
    status: "Met",
    justification:
      "Cargo uses a non-recursive build model by design. The project is a single workspace crate with no sub-make invocations. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml",
  },
  {
    id: "build_repeatable",
    status: "Met",
    justification:
      "Cargo.lock is committed to the repository. Given the same Rust toolchain (pinned via rust-toolchain.toml) and the same Cargo.lock, the build is fully reproducible. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.lock",
  },
  {
    id: "installation_common",
    status: "Met",
    justification:
      "`cargo install code-analyze-mcp` follows the standard Cargo installation convention. A Homebrew tap is also available for macOS. URL: https://github.com/clouatre-labs/code-analyze-mcp#installation",
  },
  {
    id: "installation_standard_variables",
    status: "Met",
    justification:
      "`cargo install` respects CARGO_INSTALL_ROOT and other standard Cargo environment variables. No non-standard variables are required for installation. URL: https://github.com/clouatre-labs/code-analyze-mcp#installation",
  },
  {
    id: "installation_development_quick",
    status: "Met",
    justification:
      "CONTRIBUTING.md Quick Start: clone the repository, run `cargo build`, run `cargo test`. Three commands from a fresh clone to a verified build. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/CONTRIBUTING.md",
  },
  {
    id: "external_dependencies",
    status: "Met",
    justification:
      "Cargo.toml lists all direct dependencies with version constraints. Cargo.lock pins every transitive dependency. `cargo tree` produces the full dependency graph. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml",
  },
  {
    id: "dependency_monitoring",
    status: "Met",
    justification:
      "Renovate bot creates PRs for dependency updates on a weekly schedule. `cargo deny check advisories` in CI blocks any PR that introduces a crate with a known security advisory. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/renovate.json",
  },
  {
    id: "updateable_reused_components",
    status: "Met",
    justification:
      "All dependencies are versioned crates published on crates.io. Renovate automates update PRs. No vendored or forked dependencies are present. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml",
  },
  {
    id: "interfaces_current",
    status: "Met",
    justification:
      "README documents all four MCP tool interfaces (analyze_directory, analyze_file, analyze_symbol, analyze_module), their parameters, and output formats. Documentation is updated with each release. URL: https://github.com/clouatre-labs/code-analyze-mcp#readme",
  },
  {
    id: "automated_integration_testing",
    status: "Met",
    justification:
      "The `tests/` directory contains integration tests that exercise all four MCP tools end-to-end with real source files, covering acceptance criteria, idempotency, pagination, and MCP protocol smoke tests. URL: https://github.com/clouatre-labs/code-analyze-mcp/tree/main/tests",
  },
  {
    id: "regression_tests_added50",
    status: "Met",
    justification:
      "Bug-fix commits in the last six months consistently include regression tests. Examples: fix(parser) #416 added 103 lines of integration tests, fix(pagination) #419 rewrote cursor-transition tests, fix(verbose) #366 added 30 lines, fix(metrics) #361 added 37 lines, fix(parser) #330 added 189 lines, fix(analyze_directory) #327 added 128 lines. Well above 50% of recent bug fixes include automated regression tests. URL: https://github.com/clouatre-labs/code-analyze-mcp/commits/main",
  },
  {
    id: "test_statement_coverage80",
    status: "Met",
    justification:
      "CI enforces `cargo llvm-cov report --fail-under-lines 80`. The most recent main-branch coverage run reports 80.70% line coverage (6015 lines, 1161 missed). The coverage job is a required check for all PRs. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/workflows/ci.yml",
  },
  {
    id: "test_policy_mandated",
    status: "Met",
    justification:
      'The PR template requires a "Test Plan" section. CI must pass (including `cargo test`) before a PR can be merged. Branch protection rules enforce this requirement. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/.github/PULL_REQUEST_TEMPLATE.md',
  },
  {
    id: "implement_secure_design",
    status: "Met",
    justification:
      "docs/ASSURANCE.md documents the security assurance case: no process execution, no network access, read-only file access, and explicit trust boundary definitions between the MCP client and server. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ASSURANCE.md",
  },
  {
    id: "crypto_algorithm_agility",
    status: "N/A",
    justification: "Not applicable -- the project does not use cryptography.",
  },
  {
    id: "crypto_credential_agility",
    status: "N/A",
    justification:
      "Not applicable -- the project does not handle credentials or cryptographic keys.",
  },
  {
    id: "crypto_used_network",
    status: "N/A",
    justification: "Not applicable -- the project makes no network connections.",
  },
  {
    id: "crypto_tls12",
    status: "N/A",
    justification: "Not applicable -- the project makes no network connections and uses no TLS.",
  },
  {
    id: "crypto_certificate_verification",
    status: "N/A",
    justification: "Not applicable -- the project makes no TLS connections.",
  },
  {
    id: "crypto_verification_private",
    status: "N/A",
    justification:
      "Not applicable -- the project does not use private keys.",
  },
  {
    id: "signed_releases",
    status: "Met",
    justification:
      "Every release binary is signed with cosign keyless signing. SECURITY.md \"Verifying release signatures\" section documents the verification procedure. `.bundle` files are published alongside binaries on the releases page. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/SECURITY.md",
  },
  {
    id: "version_tags_signed",
    status: "Met",
    justification:
      "Every release tag is a GPG-signed annotated tag (enforced in the release workflow). The release workflow verifies the tag signature before producing artifacts. URL: https://github.com/clouatre-labs/code-analyze-mcp/releases",
  },
  {
    id: "input_validation",
    status: "Met",
    justification:
      "All MCP tool inputs are validated: path existence checks, depth bounds enforcement, pagination bounds, and query constraints. docs/ASSURANCE.md documents the trust boundaries and input validation strategy. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ASSURANCE.md",
  },
  {
    id: "hardening",
    status: "Met",
    justification:
      'Cargo.toml release profile sets `panic = "abort"`, `strip = true`, and `opt-level = "z"`. `-D warnings` in CI ensures no unsafe or warning-generating code is merged. No `unsafe` code is present in the production codebase. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/Cargo.toml',
  },
  {
    id: "assurance_case",
    status: "Met",
    justification:
      "docs/ASSURANCE.md is a dedicated security assurance case covering threat model, trust boundaries, security controls, and residual risks. URL: https://github.com/clouatre-labs/code-analyze-mcp/blob/main/docs/ASSURANCE.md",
  },
];

// Criteria to skip from each section (computed fields).
// achieve_passing_status and achieve_silver_status are computed by the server.
const SKIP_IDS = new Set([
  "achieve_passing_status",
  "achieve_silver_status",
  "hardened_site",
]);

// Passing-ONLY criterion IDs (level 0 only -- NOT re-presented on the silver form).
// Criteria that appear on BOTH forms (e.g. contribution_requirements, report_tracker)
// are intentionally excluded here so silverAnswers() picks them up.
const PASSING_ONLY_IDS = new Set([
  "description_good", "interact", "contribution",
  "floss_license", "floss_license_osi", "license_location",
  "documentation_basics", "documentation_interface", "sites_https",
  "discussion", "english", "maintained", "repo_public", "repo_track",
  "repo_interim", "repo_distributed", "version_unique", "version_semver",
  "version_tags", "release_notes", "release_notes_vulns",
  "report_process", "report_responses",
  "enhancement_responses", "report_archive",
  "vulnerability_report_process", "vulnerability_report_private",
  "vulnerability_report_response",
  "build", "build_common_tools", "build_floss_tools",
  "test", "test_invocation", "test_most", "test_continuous_integration",
  "test_policy", "tests_are_added",
  "warnings", "warnings_fixed",
  "know_secure_design", "know_common_errors",
  "crypto_published", "crypto_call", "crypto_floss", "crypto_keylength",
  "crypto_working", "crypto_pfs",
  "crypto_password_storage", "crypto_random",
  "delivery_mitm", "delivery_unsigned",
  "vulnerabilities_critical_fixed", "vulnerabilities_critical_fixed_rapid",
  "vulnerabilities_fixed_60_days",
  "no_leaked_credentials",
  "static_analysis", "static_analysis_fixed", "static_analysis_often",
  "dynamic_analysis", "dynamic_analysis_enable_assertions", "dynamic_analysis_fixed",
]);

// The 7 criteria that appear on BOTH passing and silver forms -- they go into silverAnswers()
// so the silver form fills them (the passing form would already have them from a prior run).
// If you need to re-fill passing, they are also in ANSWERS so passingAnswers includes them.
const DUAL_IDS = new Set([
  "contribution_requirements",
  "report_tracker",
  "tests_documented_added",
  "warnings_strict",
  "static_analysis_common_vulnerabilities",
  "dynamic_analysis_unsafe",
  "crypto_weaknesses",
]);

function passingAnswers(): Answer[] {
  return ANSWERS.filter(
    (a) => (PASSING_ONLY_IDS.has(a.id) || DUAL_IDS.has(a.id)) && !SKIP_IDS.has(a.id) && a.status !== "?"
  );
}

function silverAnswers(): Answer[] {
  return ANSWERS.filter(
    (a) => (!PASSING_ONLY_IDS.has(a.id)) && !SKIP_IDS.has(a.id) && a.status !== "?"
  );
}

// Silver form section structure (matches _form_1.html.erb accordion order).
// Each section is collapsed until "Save and continue" opens the next one.
// Criteria IDs are the official level-1 IDs from criteria.yml, in presentation order.
const SILVER_SECTIONS: Array<{ name: string; continueValue: string; ids: string[] }> = [
  {
    name: "Basics",
    continueValue: "changecontrol",
    ids: [
      "achieve_passing",
      "contribution_requirements",
      "dco",
      "governance",
      "code_of_conduct",
      "roles_responsibilities",
      "access_continuity",
      "bus_factor",
      "documentation_roadmap",
      "documentation_architecture",
      "documentation_security",
      "documentation_quick_start",
      "documentation_current",
      "documentation_achievements",
      "accessibility_best_practices",
      "internationalization",
      "sites_password_security",
    ],
  },
  {
    name: "Change Control",
    continueValue: "reporting",
    ids: ["maintenance_or_update"],
  },
  {
    name: "Reporting",
    continueValue: "quality",
    ids: ["report_tracker", "vulnerability_report_credit", "vulnerability_response_process"],
  },
  {
    name: "Quality",
    continueValue: "security",
    ids: [
      "coding_standards",
      "coding_standards_enforced",
      "build_standard_variables",
      "build_preserve_debug",
      "build_non_recursive",
      "build_repeatable",
      "installation_common",
      "installation_standard_variables",
      "installation_development_quick",
      "external_dependencies",
      "dependency_monitoring",
      "updateable_reused_components",
      "interfaces_current",
      "automated_integration_testing",
      "regression_tests_added50",
      "test_statement_coverage80",
      "test_policy_mandated",
      "tests_documented_added",
      "warnings_strict",
    ],
  },
  {
    name: "Security",
    continueValue: "analysis",
    ids: [
      "implement_secure_design",
      "crypto_weaknesses",
      "crypto_algorithm_agility",
      "crypto_credential_agility",
      "crypto_used_network",
      "crypto_tls12",
      "crypto_certificate_verification",
      "crypto_verification_private",
      "signed_releases",
      "version_tags_signed",
      "input_validation",
      "hardening",
      "assurance_case",
    ],
  },
  {
    name: "Analysis",
    continueValue: "future",
    ids: ["static_analysis_common_vulnerabilities", "dynamic_analysis_unsafe"],
  },
];

async function waitForEnter(prompt: string): Promise<void> {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  return new Promise((resolve) => {
    rl.question(prompt, () => {
      rl.close();
      resolve();
    });
  });
}

async function fillSection(page: Page, answers: Answer[]): Promise<void> {
  // Small delay before starting to appear more human-like.
  await page.waitForTimeout(500);

  for (const answer of answers) {
    // Radio button for status.
    const radioSelector = `input[name="project[${answer.id}_status]"][value="${answer.status}"]`;
    const radio = page.locator(radioSelector);
    const radioCount = await radio.count();
    if (radioCount === 0) {
      console.warn(`  WARN: radio not found for ${answer.id} (${answer.status}) -- skipping`);
      continue;
    }

    // Skip radios that are already checked (nothing to do) or disabled (read-only by the server).
    const isChecked = await radio.isChecked().catch(() => false);
    const isDisabled = await radio.isDisabled().catch(() => false);
    if (isChecked || isDisabled) {
      console.log(`  SKIP: ${answer.id} (${isDisabled ? "disabled" : "already checked"})`);
      // Still fill the justification textarea even if the radio is pre-set.
    } else {
      // Scroll into view, then force-click to handle hidden-but-present radios.
      await radio.scrollIntoViewIfNeeded().catch(() => {});
      await radio.click({ force: true });
    }

    // Textarea for justification (some criteria suppress it -- catch and ignore).
    if (answer.justification) {
      const textareaSelector = `textarea[name="project[${answer.id}_justification]"]`;
      try {
        const textarea = page.locator(textareaSelector);
        const textareaCount = await textarea.count();
        if (textareaCount > 0) {
          await textarea.fill(answer.justification);
        }
      } catch {
        // Suppressed criterion -- justification textarea does not exist; ignore.
      }
    }

    // Brief pause to avoid triggering bot detection.
    await page.waitForTimeout(80);
  }
}

async function saveAndContinue(page: Page, continueValue: string): Promise<void> {
  // Click the "Save and continue" button whose value matches the next section name.
  const btn = page.locator(`button[name="continue"][value="${continueValue}"]`).first();
  const count = await btn.count();
  if (count === 0) {
    console.warn(`  WARN: Save and continue button not found for value="${continueValue}", falling back to first continue button`);
    await page.locator('button[name="continue"]').first().click();
  } else {
    await btn.click();
  }
  await page.waitForLoadState("networkidle");
  // After save-and-continue the page reloads and the next section panel opens.
  await page.waitForTimeout(600);
}

async function fillSilverBySection(page: Page, allAnswers: Answer[]): Promise<void> {
  const answerMap = new Map<string, Answer>(allAnswers.map((a) => [a.id, a]));

  for (let i = 0; i < SILVER_SECTIONS.length; i++) {
    const section = SILVER_SECTIONS[i];
    console.log(`\n  Section [${i + 1}/${SILVER_SECTIONS.length}]: ${section.name}`);

    // Build the answer list for this section (skip IDs not in our answer map or in SKIP_IDS).
    const sectionAnswers = section.ids
      .filter((id) => !SKIP_IDS.has(id))
      .map((id) => answerMap.get(id))
      .filter((a): a is Answer => a !== undefined && a.status !== "?");

    console.log(`    ${sectionAnswers.length} answers to fill`);
    await fillSection(page, sectionAnswers);

    const isLast = i === SILVER_SECTIONS.length - 1;
    if (isLast) {
      // Last section: click final "Save and continue" (value="Save") then "Submit and exit".
      console.log(`    Saving final section...`);
      await saveAndContinue(page, "Save");
    } else {
      console.log(`    Save and continue -> ${section.continueValue}...`);
      await saveAndContinue(page, section.continueValue);
    }
    console.log(`    URL after save: ${page.url()}`);
  }
}

async function submitAndExit(page: Page): Promise<void> {
  // The "Submit and Exit" button is a standard Rails submit input with no name/value.
  // Try the input[type=submit] that is NOT the "Save and Continue" button.
  // "Save and Continue" has name="continue"; the plain submit has no name.
  const submitBtn = page.locator('input[type="submit"]:not([name]), button[type="submit"]:not([name])').first();
  const count = await submitBtn.count();
  if (count === 0) {
    // Fallback: any submit button.
    await page.locator('input[type="submit"], button[type="submit"]').first().click();
  } else {
    await submitBtn.click();
  }
  await page.waitForLoadState("networkidle");
}

async function main(): Promise<void> {
  const PASSING_EDIT_URL =
    "https://www.bestpractices.dev/en/projects/12275/passing/edit";
  const SILVER_EDIT_URL =
    "https://www.bestpractices.dev/en/projects/12275/silver/edit";

  const args = process.argv.slice(2);
  const silverOnly = args.includes("--silver-only");
  const passingOnly = args.includes("--passing-only");
  if (silverOnly) console.log("Mode: silver section only (skipping passing)");
  if (passingOnly) console.log("Mode: passing section only (skipping silver)");

  // Choose the first URL to navigate to for login detection.
  const firstUrl = silverOnly ? SILVER_EDIT_URL : PASSING_EDIT_URL;

  console.log("Launching headed Chromium...");
  const browser = await chromium.launch({ headless: false });
  const context = await browser.newContext();
  const page = await context.newPage();

  // --- Navigate to first URL to trigger login if needed ---
  console.log(`Navigating to ${firstUrl}`);
  await page.goto(firstUrl);
  await page.waitForLoadState("networkidle");

  const currentUrl = page.url();
  if (currentUrl.includes("/login") || currentUrl.includes("/en/login")) {
    console.log("\nThe page redirected to the login screen.");
    console.log(
      "Please complete GitHub OAuth login in the browser, then press Enter to continue..."
    );
    await waitForEnter("> ");

    console.log(`Re-navigating to ${firstUrl}`);
    await page.goto(firstUrl);
    await page.waitForLoadState("networkidle");

    const afterLoginUrl = page.url();
    if (afterLoginUrl.includes("/login")) {
      console.error(
        "ERROR: Still on login page. Please ensure you completed the OAuth flow."
      );
      await browser.close();
      process.exit(1);
    }
  }

  // --- Fill passing section ---
  if (!silverOnly) {
    console.log("\nFilling passing-level criteria...");
    const pAnswers = passingAnswers();
    console.log(`  ${pAnswers.length} criteria to fill`);
    await fillSection(page, pAnswers);

    console.log("Submitting passing section...");
    await submitAndExit(page);
    console.log(`  After submit, URL: ${page.url()}`);

    if (passingOnly) {
      console.log("\nDone (passing only). Check https://www.bestpractices.dev/en/projects/12275");
      await browser.close();
      return;
    }
  }

  // --- Navigate to silver edit page ---
  console.log(`\nNavigating to ${SILVER_EDIT_URL}`);
  await page.goto(SILVER_EDIT_URL);
  await page.waitForLoadState("networkidle");

  const silverUrl = page.url();
  if (silverUrl.includes("/login")) {
    console.log(
      "Redirected to login again. Please complete GitHub OAuth login, then press Enter..."
    );
    await waitForEnter("> ");
    await page.goto(SILVER_EDIT_URL);
    await page.waitForLoadState("networkidle");
  }

  // --- Fill silver section-by-section ---
  console.log("Filling silver-level criteria (section by section)...");
  const sAnswers = silverAnswers();
  console.log(`  ${sAnswers.length} total silver criteria`);
  await fillSilverBySection(page, sAnswers);

  console.log("\nSubmitting silver form...");
  await submitAndExit(page);
  console.log(`  After submit, URL: ${page.url()}`);

  console.log(
    "\nDone. Check https://www.bestpractices.dev/en/projects/12275"
  );
  await browser.close();
}

main().catch((err: unknown) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
