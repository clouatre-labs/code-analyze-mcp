#!/usr/bin/env bash
# v9 benchmark runner
# Usage: ./run.sh <RUN_ID>
# Example: ./run.sh R01
#
# Run order is defined in run-order.txt.
# Condition mapping (blinding_map): R01=A4, R02=C2, R03=C5, ...
#
# Each run:
#   1. Substitutes placeholders in the condition system prompt
#   2. Invokes claude -p with the appropriate model and tool constraints
#   3. The agent writes the JSON report to results/runs/<RUN_ID>-report.json
#   4. Saves the session JSONL alongside the run file
#
# Environment requires: DISABLE_PROMPT_CACHING=1 (set inline below)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
RUNS_DIR="$SCRIPT_DIR/results/runs"
PROMPTS_DIR="$SCRIPT_DIR/prompts"
TARGET_REPO="/tmp/benchmark-repos/django"

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <RUN_ID>" >&2
  exit 1
fi

RUN_ID="$1"

# Blinding map: RUN_ID -> condition+rep
declare -A BLINDING_MAP=(
  [R01]=A4 [R02]=C2 [R03]=C5 [R04]=A5 [R05]=B4
  [R06]=A2 [R07]=C3 [R08]=B5 [R09]=C1 [R10]=B2
  [R11]=B1 [R12]=A3 [R13]=B3 [R14]=C4 [R15]=A1
)

if [[ -z "${BLINDING_MAP[$RUN_ID]+x}" ]]; then
  echo "Unknown RUN_ID: $RUN_ID (must be R01-R15)" >&2
  exit 1
fi

CONDITION_REP="${BLINDING_MAP[$RUN_ID]}"
CONDITION="${CONDITION_REP:0:1}"   # A, B, or C

OUTPUT_FILE="$RUNS_DIR/${RUN_ID}-report.json"
LOG_FILE="$RUNS_DIR/${RUN_ID}.log"

echo "=== v9 Benchmark Run ==="
echo "RUN_ID:    $RUN_ID"
echo "CONDITION: $CONDITION (rep: ${CONDITION_REP:1})"
echo "OUTPUT:    $OUTPUT_FILE"
echo ""

# Select system prompt and model based on condition
case "$CONDITION" in
  A)
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-a-control.md"
    MODEL="claude-sonnet-4-6"
    # Disallow MCP tools (both prefixed and unprefixed variants); native tools allowed
    DISALLOWED_TOOLS="mcp__code-analyze__analyze_directory,mcp__code-analyze__analyze_file,mcp__code-analyze__analyze_symbol,analyze_directory,analyze_file,analyze_symbol"
    ;;
  B)
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-b-treatment-haiku.md"
    MODEL="claude-haiku-4-5"
    # Disallow native file-exploration tools (Bash included to match validate.py constraints)
    DISALLOWED_TOOLS="Glob,Grep,Read,Bash"
    ;;
  C)
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-c-treatment-sonnet.md"
    MODEL="claude-sonnet-4-6"
    # Disallow native file-exploration tools (Bash included to match validate.py constraints)
    DISALLOWED_TOOLS="Glob,Grep,Read,Bash"
    ;;
  *)
    echo "Unknown condition: $CONDITION" >&2
    exit 1
    ;;
esac

# Substitute placeholders in system prompt
SYSTEM_PROMPT=$(sed \
  -e "s|TARGET_REPO_PATH|$TARGET_REPO|g" \
  -e "s|OUTPUT_PATH|$OUTPUT_FILE|g" \
  -e "s|RUN_ID|$RUN_ID|g" \
  "$SYSTEM_PROMPT_FILE")

# Append task content to user message (since task.md is piped as stdin)
TASK_CONTENT=$(cat "$PROMPTS_DIR/task.md")

echo "Starting run at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Model: $MODEL"
echo "Disallowed tools: $DISALLOWED_TOOLS"
echo ""

# Touch marker before the run so find -newer captures the session JSONL written during the run
touch /tmp/.v9-run-marker

# Record session start time to find the session JSONL later
SESSION_START_TS=$(date -u +%s)

# Execute the run
DISABLE_PROMPT_CACHING=1 claude \
  -p \
  --model "$MODEL" \
  --system-prompt "$SYSTEM_PROMPT" \
  --disallowed-tools "$DISALLOWED_TOOLS" \
  --dangerously-skip-permissions \
  "$TASK_CONTENT" \
  > "$LOG_FILE" \
  2>&1

echo ""
echo "Run completed at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Log: $LOG_FILE"

# Find the most recent session JSONL written after the pre-run marker
# Claude Code stores sessions under ~/.claude/projects/<slug>/ where slug = path with / replaced by -
SESSION_DIR="$HOME/.claude/projects/-Users-hugues-clouatre-git-clouatre-labs-code-analyze-mcp"
if [[ -d "$SESSION_DIR" ]]; then
  LATEST_SESSION=$(find "$SESSION_DIR" -name "*.jsonl" -newer /tmp/.v9-run-marker 2>/dev/null | sort -t/ -k1 | tail -1)
  if [[ -n "$LATEST_SESSION" ]]; then
    SESSION_COPY="$RUNS_DIR/${RUN_ID}-session.jsonl"
    cp "$LATEST_SESSION" "$SESSION_COPY"
    echo "Session JSONL: $SESSION_COPY"
  else
    echo "WARNING: Could not find session JSONL (run validation manually)" >&2
  fi
fi

# Check if output file was written
if [[ -f "$OUTPUT_FILE" ]]; then
  echo "Report file: $OUTPUT_FILE"
  if python3 -c "import json,sys; json.load(open('$OUTPUT_FILE'))" 2>/dev/null; then
    echo "Output: VALID JSON"
  else
    echo "Output: INVALID JSON" >&2
  fi
else
  echo "WARNING: Report file not found at $OUTPUT_FILE" >&2
  echo "Check $LOG_FILE for agent output" >&2
fi

