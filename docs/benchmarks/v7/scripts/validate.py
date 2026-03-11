#!/usr/bin/env python3
"""
validate.py: Tool isolation validation for v7 benchmark runs.

Queries goose sessions DB, extracts tool call information, and validates
tool isolation constraints:

- Condition A: code-analyze-mcp used; no developer__analyze calls
- Condition B: code-analyze-mcp used; no developer__analyze calls;
               tracks parameter_usage (summary, cursor, page_size)

Additional for Condition B:
- Extracts parameter_usage tracking from tool call inputs
- Detects summary=true, cursor presence, page_size non-default values

Usage:
    python3 validate.py --session-name v7-benchmark-R01-B5 --condition B
    python3 validate.py --session-name v7-benchmark-R01-A2 --condition A --db-path ~/.local/share/goose/sessions/sessions.db

Output: PASS/FAIL with details to stdout
"""

import argparse
import json
import re
import sqlite3
from pathlib import Path
from typing import List, Dict, Tuple, Optional


def get_db_connection(db_path: Path) -> sqlite3.Connection:
    """Connect to goose sessions database"""
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    return conn


def get_session_id(conn: sqlite3.Connection, session_name: str) -> Optional[str]:
    """Fetch session ID by name"""
    cursor = conn.cursor()
    cursor.execute("SELECT id FROM sessions WHERE name = ?", (session_name,))
    row = cursor.fetchone()
    return row['id'] if row else None


def get_session_messages(conn: sqlite3.Connection, session_id: str) -> List[Dict]:
    """Fetch all messages for a session"""
    cursor = conn.cursor()
    cursor.execute(
        "SELECT role, content_json FROM messages WHERE session_id = ? ORDER BY created_timestamp",
        (session_id,)
    )
    messages = []
    for row in cursor.fetchall():
        try:
            content = json.loads(row['content_json'])
            messages.append({
                'role': row['role'],
                'content': content
            })
        except json.JSONDecodeError:
            continue
    return messages


def extract_tool_calls(messages: List[Dict]) -> List[Dict]:
    """
    Extract tool calls from message content.

    Handles two message formats:
    - Anthropic-style: {"type": "tool_use", "name": "tool_name", "input": {...}}
    - Goose-style: {"type": "toolRequest", "toolCall": {"value": {"name": "tool_name", "arguments": {...}}},
                     "_meta": {"goose_extension": "ext_name"}}

    Returns list of dicts: [{"tool": "canonical_name", "input": {...}}, ...]
    The canonical name uses "{extension}__{name}" when extension differs from name.
    """
    tools = []

    for msg in messages:
        if msg['role'] != 'assistant':
            continue

        content = msg.get('content', [])
        if isinstance(content, list):
            for block in content:
                if not isinstance(block, dict):
                    continue

                if block.get('type') == 'tool_use':
                    tools.append({
                        'tool': block.get('name', 'unknown'),
                        'input': block.get('input', {})
                    })
                elif block.get('type') == 'toolRequest':
                    tool_call = block.get('toolCall', {})
                    value = tool_call.get('value', {}) if isinstance(tool_call, dict) else {}
                    name = value.get('name', 'unknown')
                    arguments = value.get('arguments', {})
                    meta = block.get('_meta', {})
                    ext = meta.get('goose_extension', '')
                    if ext and ext != name:
                        canonical = f'{ext}__{name}'
                    else:
                        canonical = name
                    tools.append({
                        'tool': canonical,
                        'input': arguments
                    })

    return tools


def count_tool_calls(tools: List[Dict]) -> Dict[str, int]:
    """Count tool calls by name"""
    counts = {}
    for tool in tools:
        name = tool['tool']
        counts[name] = counts.get(name, 0) + 1
    return counts


def extract_parameter_usage(tools: List[Dict]) -> Dict:
    """
    Extract parameter usage from code-analyze-mcp__analyze tool calls.

    Looks for:
    - summary=true calls
    - cursor-present calls
    - page_size-non-default calls

    Returns dict with:
    - summary_count: count of calls with summary=true
    - cursor_calls: count of calls with cursor present and non-empty
    - page_size_overrides: count of calls with page_size present and non-null
    - pagination_used: True if cursor_calls > 0
    """
    summary_count = 0
    cursor_calls = 0
    page_size_overrides = 0

    for tool_info in tools:
        tool_name = tool_info['tool']
        input_params = tool_info.get('input', {})

        # Only analyze code-analyze-mcp__analyze or similar analyze calls from the MCP
        if 'code-analyze-mcp' in tool_name or (tool_name == 'analyze' and isinstance(input_params, dict) and 'mode' in input_params):
            # Check for summary=true
            if input_params.get('summary') is True:
                summary_count += 1

            # Check for cursor present and non-empty
            cursor = input_params.get('cursor')
            if cursor is not None and cursor != '':
                cursor_calls += 1

            # Check for page_size present and non-null
            page_size = input_params.get('page_size')
            if page_size is not None:
                page_size_overrides += 1

    return {
        'summary_count': summary_count,
        'cursor_calls': cursor_calls,
        'page_size_overrides': page_size_overrides,
        'pagination_used': cursor_calls > 0
    }


def validate_condition_a(tools_by_name: Dict[str, int], tools_list: List[Dict]) -> Tuple[bool, List[str]]:
    """
    Validate Condition A:
    - code-analyze-mcp must be used
    - Native analyze must NOT be used (should be disabled in config)
    
    Goose canonical names:
    - Native analyze: "analyze" (extension == name, no prefix)
    - MCP analyze: contains "code-analyze-mcp"
    """
    issues = []
    
    # Check for code-analyze-mcp
    has_mcp = any('code-analyze-mcp' in name for name in tools_by_name)
    if not has_mcp:
        issues.append("ERROR: code-analyze-mcp__analyze not used (required for Condition A)")
    
    # Check for native analyze (name == "analyze" without code-analyze-mcp)
    has_native = any(
        name == 'analyze' and 'code-analyze-mcp' not in name
        for name in tools_by_name
    )
    if has_native:
        issues.append("ERROR: native analyze used (forbidden for Condition A; native extension should be disabled)")
    
    passed = len(issues) == 0
    return passed, issues


def validate_condition_b(tools_by_name: Dict[str, int], tools_list: List[Dict]) -> Tuple[bool, List[str], Dict]:
    """
    Validate Condition B:
    - code-analyze-mcp must be used
    - Native analyze must NOT be used (should be disabled in config)
    - Extract parameter_usage tracking
    
    Goose canonical names:
    - Native analyze: "analyze" (extension == name, no prefix)
    - MCP analyze: contains "code-analyze-mcp"
    
    Returns: (passed, issues, parameter_usage)
    """
    issues = []
    
    # Check for code-analyze-mcp
    has_mcp = any('code-analyze-mcp' in name for name in tools_by_name)
    if not has_mcp:
        issues.append("ERROR: code-analyze-mcp__analyze not used (required for Condition B)")
    
    # Check for native analyze (name == "analyze" without code-analyze-mcp)
    has_native = any(
        name == 'analyze' and 'code-analyze-mcp' not in name
        for name in tools_by_name
    )
    if has_native:
        issues.append("ERROR: native analyze used (forbidden for Condition B; native extension should be disabled)")
    
    # Extract parameter usage
    parameter_usage = extract_parameter_usage(tools_list)
    
    passed = len(issues) == 0
    return passed, issues, parameter_usage


def main():
    parser = argparse.ArgumentParser(
        description='Validate tool isolation for a v7 benchmark run'
    )
    parser.add_argument(
        '--session-name',
        required=True,
        help='Session name (e.g., v7-benchmark-R01-B5)'
    )
    parser.add_argument(
        '--condition',
        required=True,
        choices=['A', 'B'],
        help='Expected condition (A or B)'
    )
    parser.add_argument(
        '--db-path',
        type=Path,
        default=Path.home() / '.local' / 'share' / 'goose' / 'sessions' / 'sessions.db',
        help='Path to goose sessions database'
    )
    
    args = parser.parse_args()
    
    if not args.db_path.exists():
        print(f"FAIL: Database not found at {args.db_path}")
        exit(1)
    
    conn = get_db_connection(args.db_path)
    session_id = get_session_id(conn, args.session_name)
    
    if not session_id:
        print(f"FAIL: Session '{args.session_name}' not found in database")
        exit(1)
    
    messages = get_session_messages(conn, session_id)
    if not messages:
        print(f"FAIL: No messages found for session '{args.session_name}'")
        exit(1)
    
    tools = extract_tool_calls(messages)
    tools_by_name = count_tool_calls(tools)
    
    print(f"Session: {args.session_name}")
    print(f"Condition: {args.condition}")
    print(f"\nTool usage:")
    for name, count in sorted(tools_by_name.items()):
        print(f"  {name}: {count} call(s)")
    
    if args.condition == 'A':
        passed, issues = validate_condition_a(tools_by_name, tools)
        parameter_usage = None
    else:  # B
        passed, issues, parameter_usage = validate_condition_b(tools_by_name, tools)
    
    print(f"\nValidation: {'PASS' if passed else 'FAIL'}")
    
    if issues:
        for issue in issues:
            print(f"  {issue}")
    
    if parameter_usage is not None:
        print(f"\nParameter usage tracking (Condition B):")
        print(f"  summary_count: {parameter_usage['summary_count']}")
        print(f"  cursor_calls: {parameter_usage['cursor_calls']}")
        print(f"  page_size_overrides: {parameter_usage['page_size_overrides']}")
        print(f"  pagination_used: {parameter_usage['pagination_used']}")
    
    exit(0 if passed else 1)


if __name__ == '__main__':
    main()
