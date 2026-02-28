# Security Policy

## Reporting

Please report vulnerabilities privately via [GitHub's private vulnerability reporting](https://github.com/clouatre-labs/code-analyze-mcp/security/advisories/new).

Do not open public issues for sensitive matters.

## Branch Protection

The `main` branch is protected by GitHub rulesets with the following rules:

- **Required Status Checks**: All CI checks must pass before merging
- **Signed Commits**: All commits must be signed (GPG or S/MIME)
- **No Force Push**: History cannot be rewritten on main
- **No Deletion**: The main branch cannot be deleted
