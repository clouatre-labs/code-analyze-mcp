# Lightweight AB Harness for Regression Detection

## Purpose

The AB harness is a lightweight methodology for detecting performance and correctness regressions after changes to `src/` or tool output schemas. It measures MCP tool effectiveness through proxy metrics extracted from JSONL event logs, enabling 20-35 minute regression assessments without human scoring. It serves as a gating mechanism before escalating to full v12 or other comprehensive benchmarks.

## When to Run

Run the AB harness:
- **After any merge to main touching `src/` or tool output schemas**
- **Before/after refactoring session management or MetricEvent structure**
- **Before proposing breaking changes to tool result formats**

Do NOT run for documentation-only or dependency updates.

## Prerequisites

- MetricEvent struct includes `session_id` (Option<String>) and `seq` (Option<u32>) fields
- JSONL metrics files are written to `$XDG_DATA_HOME/code-analyze-mcp/` during tool invocations
- Same Django task as v12 (fictitious app with custom User model and profile_tier/external_sso_id/last_sync_at fields)

## Setup

Perform 6 total runs across two conditions:

**Control (current main):**
1. Build and install: `cargo install --path . --profile release`
2. Run 3 identical passes with the same Django auth migration task
3. Collect JSONL metrics files from all 3 passes

**Treatment (proposed change):**
1. Apply proposed changes to src/ or schema
2. Build and install: `cargo install --path . --profile release`
3. Run 3 identical passes with the same Django auth migration task
4. Collect JSONL metrics files from all 3 passes

Save metrics by session_id for grouping: `metrics-YYYY-MM-DD.jsonl` files contain all events from a given day; filter by session_id to isolate each run's events.

## Proxy Metrics

Measure the following 3 metrics per session (3 control runs + 3 treatment runs = 6 data points per metric):

| Metric | Source | Direction | Rationale |
|---|---|---|---|
| `research_calls_per_session` | Count of events where `tool="analyze_directory" OR tool="analyze_file"` per session | Lower is better | Fewer exploratory calls indicate faster, more focused analysis |
| `error_rate_per_session` | Count of events where `tool="analyze_module"` and `result="error"` divided by total `analyze_module` events per session | Lower is better | Currently only `analyze_module` emits `result="error"` (directory-guard path); proxy for directory-guard failures; regression = increased rate |
| `first_tool_correct_rate` | Fraction of sessions where `seq=0` event has `tool="analyze_directory"` | Higher is better | Strong analysis starts by understanding directory structure; regression = change to this pattern |

## Metric Extraction from JSONL

Use the following `jq` commands to extract metrics. Adjust path to JSONL files as needed.

### Extract research_calls_per_session

```bash
cat metrics-YYYY-MM-DD.jsonl | jq -r 'select(.session_id != null) | .session_id' | sort | uniq -c | while read count sid; do
  research=$(cat metrics-YYYY-MM-DD.jsonl | jq -s "[.[] | select(.session_id == \"$sid\" and (.tool == \"analyze_directory\" or .tool == \"analyze_file\"))] | length" 2>/dev/null)
  echo "$sid: $research research calls"
done
```

### Extract error_rate_per_session

```bash
cat metrics-YYYY-MM-DD.jsonl | jq -r 'select(.session_id != null) | .session_id' | sort -u | while read sid; do
  total=$(cat metrics-YYYY-MM-DD.jsonl | jq -s "[.[] | select(.session_id == \"$sid\")] | length" 2>/dev/null)
  errors=$(cat metrics-YYYY-MM-DD.jsonl | jq -s "[.[] | select(.session_id == \"$sid\" and .result == \"error\")] | length" 2>/dev/null)
  if [ "$total" -gt 0 ]; then
    rate=$(echo "scale=3; $errors / $total" | bc -l)
    echo "$sid: $rate error rate ($errors/$total)"
  fi
done
```

### Extract first_tool_correct_rate

```bash
cat metrics-YYYY-MM-DD.jsonl | jq -r 'select(.session_id != null) | .session_id' | sort -u | while read sid; do
  first_tool=$(cat metrics-YYYY-MM-DD.jsonl | jq -s "[.[] | select(.session_id == \"$sid\")] | min_by(.seq) | .tool" 2>/dev/null)
  if [ "$first_tool" == "\"analyze_directory\"" ]; then
    echo "$sid: 1 (first tool is analyze_directory)"
  else
    echo "$sid: 0 (first tool is $first_tool)"
  fi
done
```

Then compute the fraction of sessions where the first tool is analyze_directory.

## Decision Rule

**Regression threshold (BLOCK merge, escalate to full v12):**
- Median `research_calls_per_session` delta (treatment - control) >= 5 **OR**
- Median `error_rate_per_session` delta (treatment - control) > +0.10

**No regression (proceed with merge):**
- Both conditions above are false

## PR Comment Template

Post a table summarizing control vs treatment results on the PR:

```markdown
## AB Harness Results

| Metric | Control Median | Treatment Median | Delta | Status |
|---|---|---|---|---|
| research_calls_per_session | [value] | [value] | [delta] | ✓ PASS / ❌ FAIL |
| error_rate_per_session | [value] | [value] | [delta] | ✓ PASS / ❌ FAIL |
| first_tool_correct_rate | [value] | [value] | [delta] | ✓ PASS |

**Conclusion:** [No regression detected / Regression detected, escalating to v12 full benchmark]
```

Example passing result:

```markdown
| research_calls_per_session | 8 | 9 | +1 | ✓ PASS |
| error_rate_per_session | 0.05 | 0.08 | +0.03 | ✓ PASS |
| first_tool_correct_rate | 100% | 100% | 0% | ✓ PASS |
```
