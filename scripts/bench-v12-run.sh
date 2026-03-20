#!/usr/bin/env bash
# v12 Benchmark Runner
# Parameterized by condition ID (A, B, C, D) and run ID.
# Condition A/B use claude-sonnet-4-6, C/D use claude-haiku-4-5.
# A/C use MCP tools, B/D use native tools.
# Validates tool isolation from session JSONL.

set -euo pipefail

# Derive paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUNS_DIR="$REPO_ROOT/docs/benchmarks/v12/results/runs"
PROMPTS_DIR="$REPO_ROOT/docs/benchmarks/v12/prompts"
MCP_CODE_ANALYZE_ONLY="$REPO_ROOT/docs/benchmarks/v12/mcp-code-analyze-only.json"

mkdir -p "$RUNS_DIR"

# Parse arguments
if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <CONDITION_ID> <RUN_ID>" >&2
  echo "CONDITION_ID: A, B, C, or D" >&2
  echo "RUN_ID: e.g. A-pilot-1, B-scored-2" >&2
  exit 1
fi

CONDITION_ID="$1"
RUN_ID="$2"

# Validate condition
if [[ ! "$CONDITION_ID" =~ ^[ABCD]$ ]]; then
  echo "ERROR: CONDITION_ID must be A, B, C, or D" >&2
  exit 1
fi

# Validate RUN_ID (safe filename characters only)
if [[ ! "$RUN_ID" =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "ERROR: RUN_ID must contain only alphanumeric characters, dots, underscores, and hyphens" >&2
  exit 1
fi

# Dispatch condition to model and tool set
case "$CONDITION_ID" in
  A)
    MODEL="claude-sonnet-4-6"
    TOOL_SET="mcp"
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-a-mcp-sonnet.md"
    ;;
  B)
    MODEL="claude-sonnet-4-6"
    TOOL_SET="native"
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-b-native-sonnet.md"
    ;;
  C)
    MODEL="claude-haiku-4-5"
    TOOL_SET="mcp"
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-c-mcp-haiku.md"
    ;;
  D)
    MODEL="claude-haiku-4-5"
    TOOL_SET="native"
    SYSTEM_PROMPT_FILE="$PROMPTS_DIR/condition-d-native-haiku.md"
    ;;
esac

# Tool isolation flags
if [[ "$TOOL_SET" == "mcp" ]]; then
  ALLOWED_TOOLS="mcp__code-analyze__analyze_directory,mcp__code-analyze__analyze_file,mcp__code-analyze__analyze_symbol,mcp__code-analyze__analyze_module"
  MCP_FLAGS="--mcp-config $MCP_CODE_ANALYZE_ONLY --strict-mcp-config"
else
  ALLOWED_TOOLS="Bash,Glob,Grep,Read,Write,ToolSearch"
  EMPTY_MCP_CONFIG=$(mktemp /tmp/bench-v12-empty-mcp.XXXXXX.json)
  echo '{"mcpServers":{}}' > "$EMPTY_MCP_CONFIG"
  MCP_FLAGS="--mcp-config $EMPTY_MCP_CONFIG --strict-mcp-config"
  trap 'rm -f "$EMPTY_MCP_CONFIG"' EXIT
fi

# Output files
OUTPUT_FILE="$RUNS_DIR/${RUN_ID}-report.json"
LOG_FILE="$RUNS_DIR/${RUN_ID}.log"

# Print header
cat <<EOF
=== v12 Benchmark Run ===
CONDITION: $CONDITION_ID
RUN_ID:    $RUN_ID
MODEL:     $MODEL
TOOL_SET:  $TOOL_SET
ALLOWED:   $ALLOWED_TOOLS
OUTPUT:    $OUTPUT_FILE
EOF

# Read and substitute system prompt
TARGET_REPO="/tmp/benchmark-repos/django"
SYSTEM_PROMPT=$(sed \
  -e "s|TARGET_REPO_PATH|$TARGET_REPO|g" \
  -e "s|OUTPUT_PATH|$OUTPUT_FILE|g" \
  -e "s|RUN_ID_PLACEHOLDER|$RUN_ID|g" \
  "$SYSTEM_PROMPT_FILE")
TASK_CONTENT=$(cat "$PROMPTS_DIR/task.md")

# Tool isolation validation function
validate_tool_isolation() {
  local session_file="$1"
  local expected_tool_set="$2"  # "mcp" or "native"

  /opt/homebrew/bin/python3.14 - "$session_file" "$expected_tool_set" << 'PYEOF'
import json, sys

session_file = sys.argv[1]
expected_tool_set = sys.argv[2]

MCP_TOOLS = {
    "mcp__code-analyze__analyze_directory",
    "mcp__code-analyze__analyze_file",
    "mcp__code-analyze__analyze_symbol",
    "mcp__code-analyze__analyze_module",
}
NATIVE_TOOLS = {"Bash", "Glob", "Grep", "Read", "Write", "ToolSearch"}

tools_used = set()
with open(session_file) as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        # Look for tool_use in assistant messages
        if entry.get("type") == "assistant":
            for block in entry.get("message", {}).get("content", []):
                if isinstance(block, dict) and block.get("type") == "tool_use":
                    tools_used.add(block["name"])

print(f"Tools used: {sorted(tools_used)}")

if expected_tool_set == "mcp":
    forbidden_used = tools_used & NATIVE_TOOLS
    if forbidden_used:
        print(f"ISOLATION FAIL: native tools used in MCP condition: {forbidden_used}", file=sys.stderr)
        sys.exit(1)
    mcp_used = tools_used & MCP_TOOLS
    print(f"MCP tools used: {sorted(mcp_used)}")
    print("ISOLATION PASS: no native tools used")
elif expected_tool_set == "native":
    forbidden_used = tools_used & MCP_TOOLS
    if forbidden_used:
        print(f"ISOLATION FAIL: MCP tools used in native condition: {forbidden_used}", file=sys.stderr)
        sys.exit(1)
    native_used = tools_used & NATIVE_TOOLS
    print(f"Native tools used: {sorted(native_used)}")
    print("ISOLATION PASS: no MCP tools used")
PYEOF
}

# Session capture setup
touch /tmp/.v12-run-marker
_REPO_SLUG="${REPO_ROOT//\//-}"
SESSION_DIR="${CLAUDE_SESSION_DIR:-$HOME/.claude/projects/${_REPO_SLUG}}"

# Claude invocation
echo "Starting run at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
DISABLE_PROMPT_CACHING=1 claude \
  -p \
  --model "$MODEL" \
  --system-prompt "$SYSTEM_PROMPT" \
  $MCP_FLAGS \
  --allowedTools "$ALLOWED_TOOLS" \
  --dangerously-skip-permissions \
  "$TASK_CONTENT" \
  > "$OUTPUT_FILE" \
  2> "$LOG_FILE"
echo "Run completed at $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Session JSONL capture and validation
if [[ -d "$SESSION_DIR" ]]; then
  mapfile -t _sessions < <(find "$SESSION_DIR" -name "*.jsonl" -newer /tmp/.v12-run-marker 2>/dev/null)
  if (( ${#_sessions[@]} > 0 )); then
    LATEST_SESSION=$(ls -t "${_sessions[@]}" 2>/dev/null | head -1)
  else
    LATEST_SESSION=""
  fi
  if [[ -n "$LATEST_SESSION" ]]; then
    SESSION_COPY="$RUNS_DIR/${RUN_ID}-session.jsonl"
    cp "$LATEST_SESSION" "$SESSION_COPY"
    echo "Session JSONL: $SESSION_COPY"
    # Run tool isolation validation
    validate_tool_isolation "$SESSION_COPY" "$TOOL_SET"
  else
    echo "WARNING: Could not find session JSONL" >&2
  fi
fi

# Output validation
if [[ -f "$OUTPUT_FILE" ]]; then
  echo "Report file: $OUTPUT_FILE"
  if /opt/homebrew/bin/python3.14 -c "import json,sys; json.load(open('$OUTPUT_FILE'))" 2>/dev/null; then
    echo "Output: VALID JSON"
  else
    echo "Output: INVALID JSON" >&2
  fi
else
  echo "WARNING: Report file not found at $OUTPUT_FILE" >&2
  echo "Check $LOG_FILE for agent output" >&2
fi
