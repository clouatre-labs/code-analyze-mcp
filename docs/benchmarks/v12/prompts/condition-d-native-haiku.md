[SYSTEM PROMPT BEGIN - Condition D: Haiku + native tools]

You are a code analysis agent. Your task is to analyze a Django repository and produce a migration plan.

Repository: django/django at commit 6b90f8a8d6994dc62cd91dde911fe56ec3389494

ALLOWED TOOLS: Glob, Grep, Read, Bash
FORBIDDEN TOOLS: analyze_directory, analyze_file, analyze_symbol, analyze_module, and any other tools not listed above

Turn budget: 30 turns maximum, at most 10 tool calls total.

[SYSTEM PROMPT END - Condition D: Haiku + native tools]

## Task: Django Auth Migration

You are helping migrate a fictitious Django application to use a clean contrib.auth integration.

The application has a custom User model with three extra fields that have no direct equivalent in Django's built-in auth:
- profile_tier (CharField, choices=['free','pro','enterprise'])
- external_sso_id (CharField, unique=True, nullable)
- last_sync_at (DateTimeField, nullable)

The current model extends AbstractUser but the team wants to migrate to AbstractBaseUser for finer control.

Your task:
1. Identify the exact files and line numbers in django/contrib/auth/ that define AbstractBaseUser, AbstractUser, and the migration framework integration points.
2. Map which contrib.auth fields correspond to the custom app's existing fields and which fields (profile_tier, external_sso_id, last_sync_at) have NO direct equivalent.
3. Produce a migration plan that addresses all 3 unmappable fields, citing the specific django/contrib/auth/ files and line numbers where the integration points are.

Output must be valid JSON matching this schema:
```json
{
  "run_id": "<condition>-<pilot|scored>-<N>",
  "condition": "A|B|C|D",
  "auth_module_map": [{"file": "path/relative/to/django", "role": "description"}],
  "migration_trace": ["step 1 with file:line", "step 2 with file:line", ...],
  "unmappable_fields": [
    {"field": "profile_tier", "reason": "...", "migration_strategy": "...", "evidence": "file:line"},
    {"field": "external_sso_id", "reason": "...", "migration_strategy": "...", "evidence": "file:line"},
    {"field": "last_sync_at", "reason": "...", "migration_strategy": "...", "evidence": "file:line"}
  ],
  "tool_calls_total": 0
}
```
