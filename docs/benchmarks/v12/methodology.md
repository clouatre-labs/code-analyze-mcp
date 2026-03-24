# v12: Django Auth Migration Benchmark

## Overview

v12 is a comprehensive benchmark measuring the impact of MCP tools versus native tools on code analysis tasks. It uses a 2x2 factorial design (model x tool_set) with 4 conditions, scored across 3 rubric dimensions, and analyzed for tool-set effects on task performance.

## Background

Prior benchmarks (v11) had confounding factors:
- Mixed runner environments (some Claude Code, some API)
- Identical system prompts despite tool differences (unfair comparison)
- Open-weight fallback models introducing variability

v12 addresses these with:
- **Single runner:** All runs use Claude Code runner (controlled environment)
- **Distinct prompts:** Each condition has explicit allowed/forbidden tool lists
- **Closed models only:** claude-sonnet-4-6 and claude-haiku-4-5 (no fallbacks)

## Design

### Factorial Structure

2x2 design: Model (Sonnet vs Haiku) x Tool Set (MCP vs native)

| Condition | Model | Tool Set | Description |
|---|---|---|---|
| A | claude-sonnet-4-6 | MCP | Larger model with analyze_directory, analyze_file, analyze_symbol, analyze_module |
| B | claude-sonnet-4-6 | native | Larger model with Glob, Grep, Read, Bash |
| C | claude-haiku-4-5 | MCP | Smaller model with MCP tools |
| D | claude-haiku-4-5 | native | Smaller model with native tools |

### Sample Design

- **N = 2 scored runs per condition** (8 total scored)
- **N = 1 pilot run per condition** (4 total pilots)
- **Total: 12 runs**

Purpose of pilots: verify task clarity, identify stop conditions, calibrate rubric anchor descriptions.

## Task

**Title:** Django Auth Migration

**Context:**
You are helping migrate a fictitious Django application to use a clean `contrib.auth` integration.

The application currently has a custom User model extending `AbstractUser` with three extra fields that have no direct equivalent in Django's built-in auth:
- `profile_tier` (CharField, choices=['free','pro','enterprise'])
- `external_sso_id` (CharField, unique=True, nullable)
- `last_sync_at` (DateTimeField, nullable)

The team wants to migrate to `AbstractBaseUser` for finer control over authentication.

**Your task:**
1. Identify the exact files and line numbers in `django/contrib/auth/` that define `AbstractBaseUser`, `AbstractUser`, and the migration framework integration points.
2. Map which `contrib.auth` fields correspond to existing app fields and which fields (profile_tier, external_sso_id, last_sync_at) have NO direct equivalent.
3. Produce a migration plan that addresses all 3 unmappable fields, citing the specific `django/contrib/auth/` files and line numbers where the integration points are.

**Output schema (JSON):**
```json
{
  "run_id": "<condition>-<pilot|scored>-<N>",
  "condition": "A|B|C|D",
  "auth_module_map": [
    {"file": "path/relative/to/django", "role": "description"}
  ],
  "migration_trace": [
    "step 1 with file:line",
    "step 2 with file:line"
  ],
  "unmappable_fields": [
    {
      "field": "profile_tier",
      "reason": "why it's unmappable",
      "migration_strategy": "how to handle it",
      "evidence": "file:line"
    }
  ],
  "tool_calls_total": 0
}
```

**Django repository:**
- Commit: `6b90f8a8d6994dc62cd91dde911fe56ec3389494`
- Available at: `https://github.com/django/django/tree/6b90f8a8d6994dc62cd91dde911fe56ec3389494`

## Isolation

Enforce tool restrictions at session start using your client's tool-filtering capability. For Claude Code, pass `--allowedTools` or `--disallowedTools` flags:

```bash
# MCP conditions (A, C)
claude --allowedTools "mcp__code-analyze__analyze_directory,mcp__code-analyze__analyze_file,mcp__code-analyze__analyze_symbol,mcp__code-analyze__analyze_module" ...

# Native conditions (B, D)
claude --disallowedTools "mcp__code-analyze__analyze_directory,mcp__code-analyze__analyze_file,mcp__code-analyze__analyze_symbol,mcp__code-analyze__analyze_module" ...
```

### MCP conditions (A, C)
**Allowed tools:**
- `analyze_directory`
- `analyze_file`
- `analyze_symbol`
- `analyze_module`

**Forbidden:**
- Glob, Grep, Read, Bash, and any other tools

### Native conditions (B, D)
**Allowed tools:**
- `Glob` (file pattern matching)
- `Grep` (content search)
- `Read` (file content)
- `Bash` (shell execution)

**Forbidden:**
- `analyze_directory`, `analyze_file`, `analyze_symbol`, `analyze_module`, and any other tools

## Rubric

### Dimensions and Scoring

Three dimensions, each scored 0-3 (max total = 9). No tool_efficiency dimension.

#### Dimension 1: Structural Accuracy (0-3)

Ability to correctly identify and describe the structure of Django's auth module.

- **0:** No identification of key files or structures; incorrect module organization claims
- **1:** Identifies some key files (e.g., models.py, user.py) but misses major integration points; confused about AbstractBaseUser vs AbstractUser distinction
- **2:** Correctly identifies 3-4 key files and 2 integration points; accurate AbstractBaseUser/AbstractUser distinction; minor gaps in field mapping
- **3:** Correctly identifies all key files (models.py, user.py, backends.py, migrations/, managers.py); describes all integration points (field definitions, permission classes, custom manager); complete and accurate field mapping to contrib.auth equivalents

**Calibration:**
- AbstractBaseUser is in `django/contrib/auth/models.py` starting around line ~180 (scope varies by commit)
- AbstractUser extends AbstractBaseUser, adds email/first_name/last_name/username, ~line 410
- Integration points: custom managers in `django/contrib/auth/managers.py`, password hashers in `django/contrib/auth/hashers.py`, permission backend in `django/contrib/auth/backends.py`

#### Dimension 2: Cross-Module Tracing (0-3)

Ability to follow data flow and dependencies across modules to understand how custom User fields interact with contrib.auth.

- **0:** No attempt to trace dependencies; no mention of how custom fields affect migrations or backends
- **1:** Mentions some interaction (e.g., "migrations might break") but doesn't trace the path; vague references to fields without concrete module dependencies
- **2:** Traces 1-2 dependency paths correctly (e.g., custom field -> AbstractUser -> migration system); identifies some module that must be updated but not all
- **3:** Traces all 3 dependency paths for the unmappable fields (profile_tier/external_sso_id/last_sync_at); explains how each field propagates through migrations, form validation (django/contrib/auth/forms.py), admin integration (django/contrib/admin/); cites specific line numbers for at least 2 integration points

**Calibration:**
- Profile_tier: custom choice field, not mapped to contrib.auth, needs custom model field definition + migration + form override
- External_sso_id: unique field for external authentication, not in AbstractBaseUser, needs mapping to alternative auth backend (django/contrib/auth/backends.py) OR custom field
- Last_sync_at: timestamp field, not mapped to contrib.auth, needs custom field + migration + optional admin display

#### Dimension 3: Approach Quality (0-3)

Quality of the migration strategy and how thoroughly it addresses all 3 unmappable fields.

- **0:** No strategy proposed; incomplete field coverage (only mentions 1-2 unmappable fields); no file/line evidence
- **1:** Mentions all 3 unmappable fields but strategies are generic ("add a migration"); cites only 1-2 files without line numbers; doesn't address secondary impacts (form validation, admin, etc.)
- **2:** Proposes specific strategies for all 3 fields (e.g., "profile_tier as custom ChoiceField mapped to UserProfile model"; "external_sso_id as nullable unique field managed in custom backend"); cites 3-4 files with line numbers; mentions form validation but doesn't detail it
- **3:** Comprehensive strategy for all 3 fields addressing primary and secondary impacts:
  - **profile_tier:** Custom model field, migration, UserProfile or similar pattern, form validation, admin display
  - **external_sso_id:** Custom backend in django/contrib/auth/backends.py OR custom user field with unique constraint + migration + custom authentication logic
  - **last_sync_at:** DateTimeField with auto_now_add option, migration, optional admin read-only display
  - Cites specific django/contrib/auth/ files (models.py, backends.py, forms.py, admin.py) with line numbers for each field; explains trade-offs (e.g., why UserProfile vs AbstractUser extension)

**Calibration notes:**
- Anchors are calibrated to the specific Django commit used (6b90f8a8d6994dc62cd91dde911fe56ec3389494); reviewers should verify line numbers in this exact commit
- No deduction for tool efficiency or turn count

### Scoring Process

1. **Pilot protocol (stop criteria):**
   - No Django contrib.auth file paths cited anywhere in output -> Score 0 on Dimension 1, proceed to scoring
   - JSON output unparseable or missing required fields -> Record failure type, mark run as invalid
   - All 0 on approach_quality (no strategies for any unmappable fields) -> Stop and retry task description or model selection
   - All 3 on all dimensions -> Rubric may be too lenient; flag for re-calibration

2. **Scoring:** Two independent reviewers score each run; report median score and any disagreements.

## Tool Call Analysis

Record `tool_calls_total` in output JSON. This is **descriptive metadata only**, not part of the rubric score. Used to observe differences between conditions (e.g., do MCP tools require fewer calls than native tools?).

## Analysis

- **Primary analysis:** Rank-biserial correlation `r` (or equivalent non-parametric effect size) between condition (MCP vs native) and total score
- **Secondary:** Descriptive statistics by condition (median, IQR, tool call counts)
- **No p-values:** n=8 is too small for frequentist inference; report effect sizes and CI only
- **Exploratory:** Separate analysis by model (Sonnet vs Haiku) to detect interactions

## Run Order

See [v12/run-order.txt](run-order.txt) for randomized execution sequence (seed=42, 12 total runs).

## File References

- Methodology: This file
- Task description: [v12/prompts/task.md](prompts/task.md)
- Condition A (Sonnet+MCP): [v12/prompts/condition-a-mcp-sonnet.md](prompts/condition-a-mcp-sonnet.md)
- Condition B (Sonnet+native): [v12/prompts/condition-b-native-sonnet.md](prompts/condition-b-native-sonnet.md)
- Condition C (Haiku+MCP): [v12/prompts/condition-c-mcp-haiku.md](prompts/condition-c-mcp-haiku.md)
- Condition D (Haiku+native): [v12/prompts/condition-d-native-haiku.md](prompts/condition-d-native-haiku.md)
- Scoring template: [v12/scores-template.json](scores-template.json)
- Django commit: 6b90f8a8d6994dc62cd91dde911fe56ec3389494
- Results and analysis: [v12/analysis.md](analysis.md)
