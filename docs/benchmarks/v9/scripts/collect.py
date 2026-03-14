#!/usr/bin/env python3
"""
collect.py: Extract session metrics from a Claude Code session JSONL file (v9 extension).

v9 extends v8 to support 3-condition benchmarks and caching-disabled runs.

Claude Code stores sessions as JSONL files under:
  ~/.claude/projects/<project-slug>/<session-id>.jsonl

Each line is a JSON object representing one message turn. Assistant turns include
tool_use blocks; tool result turns follow.

Extracts:
- input_tokens, output_tokens, total_tokens (from usage fields in assistant messages)
- wall_time_s (last message timestamp - first message timestamp)
- tool_calls_total, mcp_calls, native_calls (from tool_use blocks)
- research_calls = tool_calls_total minus (Write + Edit + system Bash calls)
- cache_write_tokens, cache_read_tokens (from usage block if present, else 0)
- valid_output (bool: does the run JSON at OUTPUT_PATH parse as valid JSON?)
- cost_usd (estimated at model pricing: Sonnet or Haiku rates as of 2025)

Note: This script expects caching-disabled runs (DISABLE_PROMPT_CACHING=1 in runner).
Cache tokens will typically be 0 but are extracted for completeness.

Usage:
    python3 collect.py --session-file ~/.claude/projects/foo/SESSION_ID.jsonl
    python3 collect.py --session-file SESSION.jsonl --output-file results/runs/R01.json --model sonnet

Output: JSON to stdout (pipe into results/runs/RXX-metrics.json)
"""

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Pricing (per million tokens, as of 2025-03)
SONNET_INPUT_COST_PER_M = 3.00
SONNET_OUTPUT_COST_PER_M = 15.00
HAIKU_INPUT_COST_PER_M = 0.80
HAIKU_OUTPUT_COST_PER_M = 4.00

MCP_TOOL_NAMES = {
    "analyze_directory", "analyze_file", "analyze_symbol",
    "mcp__code-analyze__analyze_directory",
    "mcp__code-analyze__analyze_file",
    "mcp__code-analyze__analyze_symbol",
}
NATIVE_TOOL_NAMES = {"Glob", "Grep", "Read", "Bash"}
SYSTEM_BASH_PATTERNS = {"mkdir", "cd", "git", "cat", "pwd", "touch", "rm", "cp", "mv", "ls"}
# Derived from script location so the prefix is portable across checkouts
OUTPUT_PATH_PREFIXES = (
    str(Path(__file__).resolve().parent.parent),
)
# Paths that indicate target-codebase access — Bash commands touching these are NOT exempt
TARGET_REPO_INDICATORS = ("/tmp/benchmark-repos",)


def load_jsonl(path: Path) -> List[Dict]:
    messages = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                messages.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    return messages


def extract_timestamps(messages: List[Dict]) -> Tuple[Optional[datetime], Optional[datetime]]:
    """Return (first_ts, last_ts) from message timestamps.

    Timestamps are at the top-level entry (not inside "message") in Claude Code JSONL.
    """
    timestamps = []
    for entry in messages:
        ts = entry.get("timestamp") or entry.get("created_at")
        if ts is None:
            continue
        try:
            if isinstance(ts, (int, float)):
                timestamps.append(datetime.fromtimestamp(ts, tz=timezone.utc))
            else:
                timestamps.append(datetime.fromisoformat(str(ts).replace("Z", "+00:00")))
        except (ValueError, TypeError):
            continue
    if not timestamps:
        return None, None
    return timestamps[0], timestamps[-1]


def extract_tool_calls(messages: List[Dict]) -> List[Dict]:
    """
    Extract all tool_use blocks from assistant messages.

    Claude Code JSONL wraps the message in a "message" key:
      {"type": "assistant", "message": {"role": "assistant", "content": [...]}}
    Also supports unwrapped format for compatibility.
    """
    tools = []
    for entry in messages:
        msg = entry.get("message", entry)
        if msg.get("role") != "assistant":
            continue
        content = msg.get("content", [])
        if isinstance(content, str):
            continue
        for block in content:
            if not isinstance(block, dict):
                continue
            if block.get("type") == "tool_use":
                tools.append({
                    "tool": block.get("name", "unknown"),
                    "input": block.get("input", {})
                })
    return tools


def is_output_verification_call(tool_name: str, tool_input: Dict) -> bool:
    """Detect if a Read or Bash call targets the benchmark output dir, not the codebase."""
    if tool_input is None:
        return False
    if tool_name == "Read":
        path = tool_input.get("file_path", "")
        return any(path.startswith(p) for p in OUTPUT_PATH_PREFIXES)
    if tool_name == "Bash":
        cmd = tool_input.get("command", "")
        touches_output = any(p in cmd for p in OUTPUT_PATH_PREFIXES)
        touches_target = any(t in cmd for t in TARGET_REPO_INDICATORS)
        return touches_output and not touches_target
    return False


def is_system_bash_call(tool_input: Dict) -> bool:
    """Detect if a Bash call is a housekeeping command (mkdir, cd, git, etc.)."""
    if tool_input is None:
        return False
    cmd = tool_input.get("command", "").strip()
    if not cmd:
        return False
    first_word = cmd.split()[0] if cmd.split() else ""
    return first_word in SYSTEM_BASH_PATTERNS


def categorize_tools(tools: List[Dict]) -> Tuple[int, int, int, int]:
    """Return (mcp_calls, native_calls, system_bash_calls, other_calls)."""
    mcp = 0
    native = 0
    system_bash = 0
    other = 0

    for t in tools:
        tool_name = t["tool"]
        tool_input = t.get("input", {})

        if tool_name in MCP_TOOL_NAMES:
            mcp += 1
        elif tool_name in NATIVE_TOOL_NAMES:
            if tool_name == "Bash" and is_system_bash_call(tool_input):
                system_bash += 1
            elif is_output_verification_call(tool_name, tool_input):
                other += 1  # output verification, not codebase exploration
            else:
                native += 1
        else:
            # Write, Edit, or other system tools
            other += 1

    return mcp, native, system_bash, other


def count_by_name(tools: List[Dict]) -> Dict[str, int]:
    counts: Dict[str, int] = {}
    for t in tools:
        counts[t["tool"]] = counts.get(t["tool"], 0) + 1
    return counts


def check_valid_output(output_file: Optional[Path]) -> bool:
    if output_file is None or not output_file.exists():
        return False
    try:
        with open(output_file) as f:
            json.load(f)
        return True
    except (json.JSONDecodeError, OSError):
        return False


def extract_cache_tokens(messages: List[Dict]) -> Tuple[int, int]:
    """
    Extract cache_write_tokens and cache_read_tokens from usage block.
    Returns (cache_write, cache_read); default to 0 if not present.

    Usage is in entry["message"]["usage"] in Claude Code JSONL.
    """
    cache_write = 0
    cache_read = 0
    for entry in messages:
        msg = entry.get("message", entry)
        if msg.get("role") != "assistant":
            continue
        usage = msg.get("usage") or {}
        cache_write += usage.get("cache_creation_input_tokens", 0)
        cache_read += usage.get("cache_read_input_tokens", 0)
    return cache_write, cache_read


def extract_token_usage(messages: List[Dict]) -> Tuple[int, int]:
    """
    Sum input_tokens and output_tokens across all assistant messages.

    Usage is in entry["message"]["usage"] in Claude Code JSONL.
    """
    input_tokens = 0
    output_tokens = 0
    for entry in messages:
        msg = entry.get("message", entry)
        if msg.get("role") != "assistant":
            continue
        usage = msg.get("usage") or {}
        input_tokens += usage.get("input_tokens", 0) or 0
        output_tokens += usage.get("output_tokens", 0) or 0
    return input_tokens, output_tokens


def estimate_cost(input_tokens: int, output_tokens: int, model: str) -> float:
    if model.lower() == "haiku":
        return (input_tokens / 1_000_000) * HAIKU_INPUT_COST_PER_M + (output_tokens / 1_000_000) * HAIKU_OUTPUT_COST_PER_M
    else:  # default to sonnet
        return (input_tokens / 1_000_000) * SONNET_INPUT_COST_PER_M + (output_tokens / 1_000_000) * SONNET_OUTPUT_COST_PER_M


def main():
    parser = argparse.ArgumentParser(
        description="Extract session metrics from a Claude Code JSONL session file (v9)"
    )
    parser.add_argument(
        "--session-file",
        type=Path,
        required=True,
        help="Path to Claude Code session JSONL file"
    )
    parser.add_argument(
        "--output-file",
        type=Path,
        default=None,
        help="Path to agent output JSON (for valid_output check)"
    )
    parser.add_argument(
        "--model",
        default="sonnet",
        choices=["sonnet", "haiku"],
        help="Model used (for cost calculation)"
    )

    args = parser.parse_args()

    if not args.session_file.exists():
        print(json.dumps({"error": f"Session file not found: {args.session_file}"}))
        sys.exit(1)

    messages = load_jsonl(args.session_file)
    if not messages:
        print(json.dumps({"error": "No messages found in session file"}))
        sys.exit(1)

    first_ts, last_ts = extract_timestamps(messages)
    wall_time_s = int((last_ts - first_ts).total_seconds()) if first_ts and last_ts else None

    tools = extract_tool_calls(messages)
    mcp_calls, native_calls, system_bash_calls, other_calls = categorize_tools(tools)
    tool_calls_total = len(tools)
    research_calls = mcp_calls + native_calls  # system_bash and other calls excluded

    input_tokens, output_tokens = extract_token_usage(messages)
    total_tokens = input_tokens + output_tokens

    cache_write_tokens, cache_read_tokens = extract_cache_tokens(messages)

    cost_usd = estimate_cost(input_tokens, output_tokens, args.model)

    valid_output = check_valid_output(args.output_file)

    result = {
        "session_file": str(args.session_file),
        "model": args.model,
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "total_tokens": total_tokens,
        "cache_write_tokens": cache_write_tokens,
        "cache_read_tokens": cache_read_tokens,
        "cost_usd": round(cost_usd, 6),
        "wall_time_s": wall_time_s,
        "tool_calls_total": tool_calls_total,
        "research_calls": research_calls,
        "mcp_calls": mcp_calls,
        "native_calls": native_calls,
        "system_bash_calls": system_bash_calls,
        "other_calls": other_calls,
        "tool_calls_detail": count_by_name(tools),
        "valid_output": valid_output
    }

    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
