#!/usr/bin/env python3
"""
collect.py: Extract session metrics from a Claude Code session JSONL file.

Claude Code stores sessions as JSONL files under:
  ~/.claude/projects/<project-slug>/<session-id>.jsonl

Each line is a JSON object representing one message turn. Assistant turns include
tool_use blocks; tool result turns follow.

Extracts:
- input_tokens, output_tokens, total_tokens (from usage fields in assistant messages)
- wall_time_s (last message timestamp - first message timestamp)
- tool_calls_total, mcp_calls, native_calls (from tool_use blocks)
- valid_output (bool: does the run JSON at OUTPUT_PATH parse as valid JSON?)
- cost_usd (estimated at haiku-4-5 pricing: $0.80/M input, $4.00/M output as of 2025)

Usage:
    python3 collect.py --session-file ~/.claude/projects/foo/SESSION_ID.jsonl
    python3 collect.py --session-file SESSION.jsonl --output-file results/runs/R01.json

Output: JSON to stdout (pipe into results/runs/RXX.json)
"""

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Haiku 4.5 pricing (per million tokens, as of 2025-03)
HAIKU_INPUT_COST_PER_M = 0.80
HAIKU_OUTPUT_COST_PER_M = 4.00

MCP_TOOL_NAMES = {"analyze_directory", "analyze_file", "analyze_symbol"}
NATIVE_TOOL_NAMES = {"Glob", "Grep", "Read", "Bash"}


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
    """Return (first_ts, last_ts) from message timestamps."""
    timestamps = []
    for msg in messages:
        ts = msg.get("timestamp") or msg.get("created_at")
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

    Claude Code JSONL format: each message has a "role" and "content" field.
    content is a list of blocks; tool_use blocks have {"type": "tool_use", "name": ..., "input": ...}.
    """
    tools = []
    for msg in messages:
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


def extract_token_usage(messages: List[Dict]) -> Tuple[int, int]:
    """
    Sum input_tokens and output_tokens across all assistant messages.

    Claude Code embeds usage in the message or in a trailing metadata object.
    Looks for: msg["usage"]["input_tokens"] / msg["usage"]["output_tokens"]
    Also handles top-level "input_tokens"/"output_tokens" fields.
    """
    input_tokens = 0
    output_tokens = 0
    for msg in messages:
        if msg.get("role") != "assistant":
            continue
        usage = msg.get("usage") or {}
        input_tokens += usage.get("input_tokens", 0) or msg.get("input_tokens", 0)
        output_tokens += usage.get("output_tokens", 0) or msg.get("output_tokens", 0)
    return input_tokens, output_tokens


def categorize_tools(tools: List[Dict]) -> Tuple[int, int, int]:
    """Return (mcp_calls, native_calls, other_calls)."""
    mcp = sum(1 for t in tools if t["tool"] in MCP_TOOL_NAMES)
    native = sum(1 for t in tools if t["tool"] in NATIVE_TOOL_NAMES)
    other = len(tools) - mcp - native
    return mcp, native, other


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


def estimate_cost(input_tokens: int, output_tokens: int) -> float:
    return (input_tokens / 1_000_000) * HAIKU_INPUT_COST_PER_M + (output_tokens / 1_000_000) * HAIKU_OUTPUT_COST_PER_M


def main():
    parser = argparse.ArgumentParser(
        description="Extract session metrics from a Claude Code JSONL session file"
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
    mcp_calls, native_calls, other_calls = categorize_tools(tools)
    tool_calls_total = len(tools)

    input_tokens, output_tokens = extract_token_usage(messages)
    total_tokens = input_tokens + output_tokens
    cost_usd = estimate_cost(input_tokens, output_tokens)

    valid_output = check_valid_output(args.output_file)

    result = {
        "session_file": str(args.session_file),
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
        "total_tokens": total_tokens,
        "cost_usd": round(cost_usd, 6),
        "wall_time_s": wall_time_s,
        "tool_calls_total": tool_calls_total,
        "mcp_calls": mcp_calls,
        "native_calls": native_calls,
        "other_calls": other_calls,
        "tool_calls_detail": count_by_name(tools),
        "valid_output": valid_output
    }

    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
