#!/usr/bin/env bash
# run_goose.sh: Execute one goose benchmark run for conditions D or E.
# Usage: bash scripts/run_goose.sh <RUN_ID> <CONDITION>
# Example: bash scripts/run_goose.sh R03 D

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUNS_DIR="$SCRIPT_DIR/results/runs"
PROMPTS_DIR="$SCRIPT_DIR/prompts"
TARGET_REPO="/tmp/benchmark-repos/django"

RUN_ID="$1"
CONDITION="$2"

OUTPUT_FILE="$RUNS_DIR/${RUN_ID}-report.json"
SESSION_JSONL="$RUNS_DIR/${RUN_ID}-session.jsonl"

# Select model and system prompt based on condition
case "$CONDITION" in
  D)
    MODEL="minimax/minimax-m2.5"
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-d-treatment-minimax.md"
    COLLECT_MODEL_ID="minimax/minimax-m2.5"
    ;;
  E)
    MODEL="mistralai/mistral-small-2603"
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-e-treatment-mistral.md"
    COLLECT_MODEL_ID="mistralai/mistral-small-2603"
    ;;
  *)
    echo "Unknown condition: $CONDITION" >&2
    exit 1
    ;;
esac

if [[ -z "${OPENROUTER_API_KEY:-}" ]]; then
  echo "ERROR: OPENROUTER_API_KEY must be set" >&2
  exit 1
fi

# Build combined prompt: system prompt (with substitutions) + task
SYSTEM_PROMPT=$(sed \
  -e "s|TARGET_REPO_PATH|$TARGET_REPO|g" \
  -e "s|OUTPUT_PATH|$OUTPUT_FILE|g" \
  -e "s|RUN_ID|$RUN_ID|g" \
  "$SYSTEM_PROMPT_FILE")

TASK_CONTENT=$(cat "$PROMPTS_DIR/task.md")
FULL_PROMPT="${SYSTEM_PROMPT}

---

${TASK_CONTENT}"

# Record sessions count before run to find the new session
SESSION_COUNT_BEFORE=$(sqlite3 ~/.local/share/goose/sessions/sessions.db "SELECT COUNT(*) FROM sessions;" 2>/dev/null || echo "0")

echo "=== Starting goose run: $RUN_ID (condition $CONDITION) ==="
echo "Model: $MODEL"
echo "Output: $OUTPUT_FILE"
echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# Run goose - capture output to log and also to stdout
LOG_FILE="$RUNS_DIR/${RUN_ID}.log"
GOOSE_SESSION_NAME="v10-${RUN_ID}"

DISABLE_PROMPT_CACHING=1 OPENROUTER_API_KEY="$OPENROUTER_API_KEY" goose run \
  --provider openrouter \
  --model "$MODEL" \
  --with-extension code-analyze-mcp \
  --with-builtin developer \
  --no-profile \
  --name "$GOOSE_SESSION_NAME" \
  --text "$FULL_PROMPT" \
  2>&1 | tee "$LOG_FILE"

echo ""
echo "Completed: $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Find the session ID (new session created after SESSION_COUNT_BEFORE)
SESSION_ID=$(sqlite3 ~/.local/share/goose/sessions/sessions.db \
  "SELECT id FROM sessions WHERE name = '${GOOSE_SESSION_NAME}' ORDER BY created_at DESC LIMIT 1;" 2>/dev/null)

if [[ -z "$SESSION_ID" ]]; then
  echo "ERROR: Could not find session ID for run $RUN_ID" >&2
  exit 1
fi

echo "Session ID: $SESSION_ID"

# Convert goose session to JSONL
python3 "$SCRIPT_DIR/scripts/goose_to_jsonl.py" \
  --session-id "$SESSION_ID" \
  --output "$SESSION_JSONL"

echo "Session JSONL: $SESSION_JSONL"
echo "Lines: $(wc -l < "$SESSION_JSONL")"

