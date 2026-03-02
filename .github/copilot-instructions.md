# Copilot Instructions

For universal project instructions, see `AGENTS.md` in the repo root.

## Assigning issues to Copilot

REST API method:
```
gh api repos/{owner}/{repo}/issues/{number} --method PATCH -f "assignees[]=copilot-swe-agent[bot]"
```
UI method: use the issue sidebar to assign to `copilot-swe-agent[bot]`.

## PR iteration

- Comment `@copilot` with specific feedback; agent pushes follow-up commits
- If unresolved after 2 iterations: close PR, amend issue, re-assign

## Copilot code review

Flag:
- Hallucinated APIs; verify against `Cargo.lock` and `cargo doc --open`
- Scope creep beyond the issue deliverables
- Missing error handling
- Only Rust is implemented; SymbolFocus mode is stubbed (Wave 3)
- rmcp, tree-sitter, schemars version assumptions; verify against installed versions

## Design references

- [ARCHITECTURE.md](../docs/ARCHITECTURE.md) - design, language handlers, "How to add a language"
- [AGENTS.md](../AGENTS.md) - project conventions and implementation status

## Firewall

- Copilot runs in a firewalled GitHub Actions environment; no arbitrary URLs
- If a build step needs a URL not in the allow list, document it in the PR
- Maintainer updates `.github/copilot/firewall.yml`
