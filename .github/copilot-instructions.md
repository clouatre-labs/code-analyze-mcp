# Copilot Instructions

For universal project instructions, see `AGENTS.md` in the repo root.

## Assigning issues to Copilot

REST API method:
```
gh api repos/{owner}/{repo}/issues/{number} --method PATCH -f "assignees[]=copilot-swe-agent[bot]"
```
UI method: use the issue sidebar to assign to `copilot-swe-agent[bot]`.

## PR iteration

Comment `@copilot` with specific feedback; agent pushes follow-up commits.
If the agent cannot resolve after 2 iterations: close the PR, amend the issue with clarifications, and re-assign.

## Copilot code review

Flag:
- Hallucinated APIs (methods that do not exist in the installed crate versions)
- Scope creep beyond the issue deliverables
- Missing error handling

## Firewall

Copilot coding agent runs in a firewalled GitHub Actions environment and cannot fetch arbitrary URLs.
If a build or test step needs a URL not in the default allow list, document it in the PR so the maintainer can update `.github/copilot/firewall.yml`.
