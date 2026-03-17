#!/usr/bin/env bash
# v10 benchmark runner
# Usage: ./run.sh <RUN_ID>
# Example: ./run.sh R01
#
# Run order is defined in run-order.txt.
# Condition mapping (blinding_map): R01=C4, R02=A22, R03=D3, ...
#
# Each run:
#   1. Substitutes placeholders in the condition system prompt
#   2. Invokes the appropriate runner (claude CLI for A/A2/B, goose for D/E)
#   3. The agent writes the JSON report to results/runs/<RUN_ID>-report.json
#   4. Saves the session JSONL alongside the run file
#
# Environment requires: DISABLE_PROMPT_CACHING=1 (set inline below for claude CLI runs)

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

# Blinding map: RUN_ID -> condition+rep (24 runs across 6 conditions)
declare -A BLINDING_MAP=(
  [R01]=C4  [R02]=A22 [R03]=D3  [R04]=E4
  [R05]=C2  [R06]=B3  [R07]=D2  [R08]=B4
  [R09]=C3  [R10]=D1  [R11]=A2  [R12]=C1
  [R13]=E2  [R14]=A23 [R15]=B2  [R16]=A3
  [R17]=E3  [R18]=A21 [R19]=D4  [R20]=A24
  [R21]=B1  [R22]=A1  [R23]=A4  [R24]=E1
)

if [[ -z "${BLINDING_MAP[$RUN_ID]+x}" ]]; then
  echo "Unknown RUN_ID: $RUN_ID (must be R01-R24)" >&2
  exit 1
fi

CONDITION_REP="${BLINDING_MAP[$RUN_ID]}"

# Extract CONDITION and REP: conditions can be 1 or 2 characters (A, B, C, D, E) or "A2"
if [[ "${CONDITION_REP:0:2}" == "A2" ]]; then
  CONDITION="A2"
  REP="${CONDITION_REP:2}"
else
  CONDITION="${CONDITION_REP:0:1}"
  REP="${CONDITION_REP:1}"
fi

OUTPUT_FILE="$RUNS_DIR/${RUN_ID}-report.json"
LOG_FILE="$RUNS_DIR/${RUN_ID}.log"

echo "=== v10 Benchmark Run ==="
echo "RUN_ID:    $RUN_ID"
echo "CONDITION: $CONDITION (rep: $REP)"
echo "OUTPUT:    $OUTPUT_FILE"
echo ""

# Select system prompt, model, and runner based on condition
case "$CONDITION" in
  A)
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-a-control.md"
    MODEL="claude-sonnet-4-6"
    DISALLOWED_TOOLS="mcp__code-analyze__analyze_directory,mcp__code-analyze__analyze_file,mcp__code-analyze__analyze_symbol,analyze_directory,analyze_file,analyze_symbol"
    RUNNER="claude_cli"
    ;;
  A2)
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-a2-haiku-native.md"
    MODEL="claude-haiku-4-5"
    DISALLOWED_TOOLS="mcp__code-analyze__analyze_directory,mcp__code-analyze__analyze_file,mcp__code-analyze__analyze_symbol,analyze_directory,analyze_file,analyze_symbol"
    RUNNER="claude_cli"
    ;;
  B)
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-b-treatment-haiku.md"
    MODEL="claude-haiku-4-5"
    DISALLOWED_TOOLS="Glob,Grep,Read,Bash"
    RUNNER="claude_cli"
    ;;
  C)
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-c-treatment-sonnet.md"
    MODEL="claude-sonnet-4-6"
    DISALLOWED_TOOLS="Glob,Grep,Read,Bash"
    RUNNER="claude_cli"
    ;;
  D)
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-d-treatment-minimax.md"
    MODEL="minimax/minimax-m2.5"
    RUNNER="goose"
    ;;
  E)
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-e-treatment-mistral.md"
    MODEL="mistralai/mistral-small-2603"
    RUNNER="goose"
    ;;
  *)
    echo "Unknown condition: $CONDITION" >&2
    exit 1
    ;;
esac

# Check OPENROUTER_API_KEY for goose runs (D, E)
if [[ "$RUNNER" == "goose" ]]; then
  if [[ -z "${OPENROUTER_API_KEY:-}" ]]; then
    echo "ERROR: OPENROUTER_API_KEY must be set for condition $CONDITION" >&2
    exit 1
  fi
fi

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
echo "RUNNER:    $RUNNER"
echo ""

# Execute based on runner type
if [[ "$RUNNER" == "claude_cli" ]]; then
  # Disallowed tools for claude CLI runs
  echo "Disallowed tools: $DISALLOWED_TOOLS"
  echo ""

  # Touch marker before the run so find -newer captures the session JSONL written during the run
  touch /tmp/.v10-run-marker

  # Record session start time to find the session JSONL later
  SESSION_START_TS=$(date -u +%s)

  # Execute the run with claude CLI
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
    LATEST_SESSION=$(find "$SESSION_DIR" -name "*.jsonl" -newer /tmp/.v10-run-marker 2>/dev/null | sort -t/ -k1 | tail -1)
    if [[ -n "$LATEST_SESSION" ]]; then
      SESSION_COPY="$RUNS_DIR/${RUN_ID}-session.jsonl"
      cp "$LATEST_SESSION" "$SESSION_COPY"
      echo "Session JSONL: $SESSION_COPY"
    else
      echo "WARNING: Could not find session JSONL (run validation manually)" >&2
    fi
  fi

elif [[ "$RUNNER" == "goose" ]]; then
  # Goose runner: emit instructions instead of executing
  echo "=== GOOSE SESSION REQUIRED ==="
  echo "Conditions D and E require a goose session with the model configured via OpenRouter."
  echo "MCP tools (analyze_directory, analyze_file, analyze_symbol) are only accessible via the goose agent loop."
  echo ""
  echo "Start a goose session with:"
  echo "  OPENROUTER_API_KEY=$OPENROUTER_API_KEY \\"
  echo "  goose session --provider openrouter --model $MODEL \\"
  echo "    --with-extension code-analyze"
  echo ""
  echo "In the session, provide the system prompt from:"
  echo "  $SYSTEM_PROMPT_FILE"
  echo ""
  echo "Task content from:"
  echo "  $PROMPTS_DIR/task.md"
  echo ""
  echo "Write the report to: $OUTPUT_FILE"
  echo "Save the session JSONL to: ${RUNS_DIR}/${RUN_ID}-session.jsonl"
  echo ""
  echo "Log reasoning_mode: disabled"
  exit 0
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
