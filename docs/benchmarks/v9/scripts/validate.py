#!/usr/bin/env python3
"""
validate.py: Tool isolation validation for v9 benchmark runs (3 conditions).

Reads a Claude Code session JSONL file and validates tool isolation constraints:

Condition A (control — Sonnet + native tools only):
  - MUST use at least one native tool (Glob, Grep, Read, Bash)
  - MUST NOT use MCP tools (analyze_directory, analyze_file, analyze_symbol)
  - research_calls MUST be <= 10

Condition B (treatment — Haiku + MCP tools only):
  - MUST use at least one MCP tool (analyze_directory, analyze_file, analyze_symbol)
  - MUST NOT use native file-exploration tools (Glob, Grep, Read, Bash)
  - research_calls MUST be <= 10

Condition C (treatment — Sonnet + MCP tools only):
  - MUST use at least one MCP tool (analyze_directory, analyze_file, analyze_symbol)
  - MUST NOT use native file-exploration tools (Glob, Grep, Read, Bash)
  - research_calls MUST be <= 10

Usage:
    python3 validate.py --session-file SESSION.jsonl --condition A
    python3 validate.py --session-file SESSION.jsonl --condition B
    python3 validate.py --session-file SESSION.jsonl --condition C

Output: PASS/FAIL with details to stdout. Exit 0 on pass, 1 on fail.
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Tuple

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


def extract_tool_calls(messages: List[Dict]) -> List[Tuple[str, Dict]]:
    """Return list of (tool_name, input_dict) tuples from assistant messages.

    Claude Code JSONL wraps the message in a "message" key:
      {"type": "assistant", "message": {"role": "assistant", "content": [...]}}
    """
    tools = []
    for entry in messages:
        # Support both wrapped (Claude Code JSONL) and unwrapped formats
        msg = entry.get("message", entry)
        if msg.get("role") != "assistant":
            continue
        content = msg.get("content", [])
        if isinstance(content, str):
            continue
        for block in content:
            if isinstance(block, dict) and block.get("type") == "tool_use":
                name = block.get("name", "unknown")
                inp = block.get("input", {})
                tools.append((name, inp))
    return tools


def is_output_verification_call(tool_name: str, tool_input: Dict) -> bool:
    """Detect if a Read or Bash call targets the benchmark output dir (not the codebase).

    Agents may read/verify their own output file after writing it; this is not
    codebase exploration and should not count as a native research call.
    """
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
    """Detect if a Bash call is a housekeeping command."""
    if tool_input is None:
        return False
    cmd = tool_input.get("command", "").strip()
    if not cmd:
        return False
    first_word = cmd.split()[0] if cmd.split() else ""
    return first_word in SYSTEM_BASH_PATTERNS


def count_tools(tools: List[Tuple[str, Dict]]) -> Tuple[Dict[str, int], int, int, int]:
    """
    Return (detail_counts, mcp_count, native_count, research_calls).
    research_calls = mcp + native (system_bash excluded).
    """
    detail: Dict[str, int] = {}
    mcp = 0
    native = 0

    for name, inp in tools:
        detail[name] = detail.get(name, 0) + 1

        if name in MCP_TOOL_NAMES:
            mcp += 1
        elif name in NATIVE_TOOL_NAMES:
            if name == "Bash" and is_system_bash_call(inp):
                pass  # system bash, not research
            elif is_output_verification_call(name, inp):
                pass  # reading/verifying own output, not codebase exploration
            else:
                native += 1

    research_calls = mcp + native
    return detail, mcp, native, research_calls


def validate_condition_a(tools: List[Tuple[str, Dict]]) -> Tuple[bool, List[str]]:
    """Condition A: native only, no MCP, research_calls <= 10."""
    issues = []
    detail, mcp, native, research_calls = count_tools(tools)

    has_native = native > 0
    has_mcp = mcp > 0

    if not has_native:
        issues.append("ERROR: no native tools used (Glob/Grep/Read/Bash required for Condition A)")
    if has_mcp:
        mcp_used = [t for t, _ in tools if t in MCP_TOOL_NAMES]
        issues.append(f"ERROR: MCP tools used (forbidden for Condition A): {mcp_used}")
    if research_calls > 10:
        issues.append(f"WARN: research_calls ({research_calls}) exceeds budget (10)")

    return len([i for i in issues if i.startswith("ERROR")]) == 0, issues


def validate_mcp_only_condition(tools: List[Tuple[str, Dict]], label: str) -> Tuple[bool, List[str]]:
    """Shared validator for MCP-only conditions (B and C): MCP required, no native file-exploration."""
    issues = []
    detail, mcp, native, research_calls = count_tools(tools)

    if mcp == 0:
        issues.append(f"ERROR: no MCP tools used (analyze_directory/analyze_file/analyze_symbol required for Condition {label})")
    if native > 0:
        native_used = [t for t, inp in tools if t in NATIVE_TOOL_NAMES and not (t == "Bash" and is_system_bash_call(inp))]
        if native_used:
            issues.append(f"ERROR: native file-exploration tools used (forbidden for Condition {label}): {native_used}")
    if research_calls > 10:
        issues.append(f"WARN: research_calls ({research_calls}) exceeds budget (10)")

    return len([i for i in issues if i.startswith("ERROR")]) == 0, issues


def validate_condition_b(tools: List[Tuple[str, Dict]]) -> Tuple[bool, List[str]]:
    """Condition B: MCP only, no native file-exploration, research_calls <= 10."""
    return validate_mcp_only_condition(tools, "B")


def validate_condition_c(tools: List[Tuple[str, Dict]]) -> Tuple[bool, List[str]]:
    """Condition C: MCP only, no native file-exploration, research_calls <= 10."""
    return validate_mcp_only_condition(tools, "C")


def main():
    parser = argparse.ArgumentParser(
        description="Validate tool isolation for a v9 benchmark run"
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
        choices=["A", "B", "C"],
        help="Expected condition (A, B, or C)"
    )

    args = parser.parse_args()

    if not args.session_file.exists():
        print(f"FAIL: Session file not found: {args.session_file}")
        sys.exit(1)

    messages = load_jsonl(args.session_file)
    if not messages:
        print(f"FAIL: No messages found in {args.session_file}")
        sys.exit(1)

    tools = extract_tool_calls(messages)
    detail, mcp, native, research_calls = count_tools(tools)

    print(f"Session: {args.session_file.name}")
    print(f"Condition: {args.condition}")
    print(f"\nTool usage ({len(tools)} total calls):")
    for name, count in sorted(detail.items()):
        category = "MCP" if name in MCP_TOOL_NAMES else ("native" if name in NATIVE_TOOL_NAMES else "other")
        print(f"  {name}: {count} call(s)  [{category}]")
    print(f"\nResearch calls: {research_calls} (budget: 10)")

    if args.condition == "A":
        passed, issues = validate_condition_a(tools)
    elif args.condition == "B":
        passed, issues = validate_condition_b(tools)
    else:
        passed, issues = validate_condition_c(tools)

    print(f"\nValidation: {'PASS' if passed else 'FAIL'}")
    for issue in issues:
        print(f"  {issue}")

    sys.exit(0 if passed else 1)


if __name__ == "__main__":
    main()
