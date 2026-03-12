#!/usr/bin/env python3
"""
validate.py: Tool isolation validation for v8 benchmark runs.

Reads a Claude Code session JSONL file and validates tool isolation constraints:

Condition A (control — native tools only):
  - MUST use at least one native tool (Glob, Grep, Read, Bash)
  - MUST NOT use MCP tools (analyze_directory, analyze_file, analyze_symbol)

Condition B (treatment — native + MCP preferred):
  - MUST use at least one MCP tool (analyze_directory, analyze_file, analyze_symbol)
  - Native tools are allowed as fallback

Usage:
    python3 validate.py --session-file SESSION.jsonl --condition A
    python3 validate.py --session-file SESSION.jsonl --condition B

Output: PASS/FAIL with details to stdout. Exit 0 on pass, 1 on fail.
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Tuple

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


def extract_tool_calls(messages: List[Dict]) -> List[str]:
    """Return list of tool names used in assistant messages."""
    names = []
    for msg in messages:
        if msg.get("role") != "assistant":
            continue
        content = msg.get("content", [])
        if isinstance(content, str):
            continue
        for block in content:
            if isinstance(block, dict) and block.get("type") == "tool_use":
                names.append(block.get("name", "unknown"))
    return names


def count_tools(names: List[str]) -> Dict[str, int]:
    counts: Dict[str, int] = {}
    for n in names:
        counts[n] = counts.get(n, 0) + 1
    return counts


def validate_condition_a(counts: Dict[str, int]) -> Tuple[bool, List[str]]:
    issues = []
    has_native = any(t in counts for t in NATIVE_TOOL_NAMES)
    has_mcp = any(t in counts for t in MCP_TOOL_NAMES)

    if not has_native:
        issues.append("ERROR: no native tools used (Glob/Grep/Read/Bash required for Condition A)")
    if has_mcp:
        mcp_used = [t for t in MCP_TOOL_NAMES if t in counts]
        issues.append(f"ERROR: MCP tools used (forbidden for Condition A): {mcp_used}")

    return len(issues) == 0, issues


def validate_condition_b(counts: Dict[str, int]) -> Tuple[bool, List[str]]:
    issues = []
    has_mcp = any(t in counts for t in MCP_TOOL_NAMES)

    if not has_mcp:
        issues.append("ERROR: no MCP tools used (analyze_directory/analyze_file/analyze_symbol required for Condition B)")

    return len(issues) == 0, issues


def main():
    parser = argparse.ArgumentParser(
        description="Validate tool isolation for a v8 benchmark run"
    )
    parser.add_argument(
        "--session-file",
        type=Path,
        required=True,
        help="Path to Claude Code session JSONL file"
    )
    parser.add_argument(
        "--condition",
        required=True,
        choices=["A", "B"],
        help="Expected condition (A or B)"
    )

    args = parser.parse_args()

    if not args.session_file.exists():
        print(f"FAIL: Session file not found: {args.session_file}")
        sys.exit(1)

    messages = load_jsonl(args.session_file)
    if not messages:
        print(f"FAIL: No messages found in {args.session_file}")
        sys.exit(1)

    tool_names = extract_tool_calls(messages)
    counts = count_tools(tool_names)

    print(f"Session: {args.session_file.name}")
    print(f"Condition: {args.condition}")
    print(f"\nTool usage ({len(tool_names)} total calls):")
    for name, count in sorted(counts.items()):
        category = "MCP" if name in MCP_TOOL_NAMES else ("native" if name in NATIVE_TOOL_NAMES else "other")
        print(f"  {name}: {count} call(s)  [{category}]")

    if args.condition == "A":
        passed, issues = validate_condition_a(counts)
    else:
        passed, issues = validate_condition_b(counts)

    print(f"\nValidation: {'PASS' if passed else 'FAIL'}")
    for issue in issues:
        print(f"  {issue}")

    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
