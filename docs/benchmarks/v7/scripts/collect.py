#!/usr/bin/env python3
"""
collect.py: Extract session metrics from goose sessions database.

Queries a goose session by name, extracts:
- total_tokens, input_tokens, output_tokens from sessions table
- accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens from sessions table
- wall time (last message timestamp - first message timestamp)
- tool call counts by tool name from messages.content_json
- parameter_usage (v7): summary_count, cursor_calls, page_size_overrides, pagination_used

Usage:
    python3 collect.py --session-name R01-B5
    python3 collect.py --session-name R01-B5 --db-path ~/.local/share/goose/sessions/sessions.db

Output: JSON to stdout (can be pasted into scores-template.json efficiency section)
"""

import argparse
import json
import sqlite3
from datetime import datetime
from pathlib import Path
from typing import Dict, Optional, List


def get_db_connection(db_path: Path) -> sqlite3.Connection:
    """Connect to goose sessions database"""
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    return conn


def get_session_info(conn: sqlite3.Connection, session_name: str) -> Optional[Dict]:
    """Fetch session info from sessions table"""
    cursor = conn.cursor()
    cursor.execute(
        "SELECT id, total_tokens, input_tokens, output_tokens, accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens FROM sessions WHERE name = ?",
        (session_name,)
    )
    row = cursor.fetchone()
    return dict(row) if row else None


def get_session_messages(conn: sqlite3.Connection, session_id: str) -> list:
    """Fetch all messages for a session with timestamps"""
    cursor = conn.cursor()
    cursor.execute(
        "SELECT created_timestamp, role, content_json FROM messages WHERE session_id = ? ORDER BY created_timestamp",
        (session_id,)
    )
    return [dict(row) for row in cursor.fetchall()]


def extract_tool_calls(messages: list) -> Dict[str, int]:
    """
    Extract tool calls from message content and count by tool name.

    Handles two message formats:
    - Anthropic-style: {"type": "tool_use", "name": "tool_name", ...}
    - Goose-style: {"type": "toolRequest", "toolCall": {"value": {"name": "tool_name"}},
                     "_meta": {"goose_extension": "ext_name"}}

    For goose-style, the canonical tool name is "{extension}__{name}" when extension
    differs from name, matching how goose exposes tools to the model.
    """
    counts = {}

    for msg in messages:
        if msg['role'] != 'assistant':
            continue

        try:
            content = json.loads(msg['content_json'])
        except (json.JSONDecodeError, TypeError):
            continue

        if isinstance(content, list):
            for block in content:
                if not isinstance(block, dict):
                    continue

                tool_name = None

                if block.get('type') == 'tool_use':
                    tool_name = block.get('name', 'unknown')
                elif block.get('type') == 'toolRequest':
                    tool_call = block.get('toolCall', {})
                    value = tool_call.get('value', {}) if isinstance(tool_call, dict) else {}
                    name = value.get('name', 'unknown')
                    meta = block.get('_meta', {})
                    ext = meta.get('goose_extension', '')
                    if ext and ext != name:
                        tool_name = f'{ext}__{name}'
                    else:
                        tool_name = name

                if tool_name:
                    counts[tool_name] = counts.get(tool_name, 0) + 1

    return counts


def extract_tool_calls_with_inputs(messages: list) -> List[Dict]:
    """
    Extract tool calls with their full input parameters.

    Returns list of dicts: [{"tool": "canonical_name", "input": {...}}, ...]
    """
    tools = []

    for msg in messages:
        if msg['role'] != 'assistant':
            continue

        try:
            content = json.loads(msg['content_json'])
        except (json.JSONDecodeError, TypeError):
            continue

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


def extract_parameter_usage(messages: list) -> Dict:
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
    tools = extract_tool_calls_with_inputs(messages)

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


def calculate_wall_time(messages: list) -> Optional[int]:
    """Calculate wall time in seconds from first to last message"""
    if not messages:
        return None
    
    try:
        first_ts = messages[0]['created_timestamp']
        last_ts = messages[-1]['created_timestamp']
        
        # Timestamps may be ISO format strings or Unix timestamps
        if isinstance(first_ts, str):
            first_dt = datetime.fromisoformat(first_ts.replace('Z', '+00:00'))
            last_dt = datetime.fromisoformat(last_ts.replace('Z', '+00:00'))
        else:
            first_dt = datetime.fromtimestamp(first_ts)
            last_dt = datetime.fromtimestamp(last_ts)
        
        delta = last_dt - first_dt
        return int(delta.total_seconds())
    except (ValueError, TypeError):
        return None


def categorize_tool_calls(tool_counts: Dict[str, int]) -> Dict[str, int]:
    """
    Categorize tools into analyze_calls, shell_calls, editor_calls, tree_calls.

    Handles goose-style names like:
    - analyze, code-analyze-mcp__code-analyze-mcp__analyze -> analyze
    - developer__shell, shell -> shell
    - developer__write, developer__edit, write, edit -> editor
    - developer__tree, tree -> tree

    Returns dict with keys: analyze_calls, shell_calls, editor_calls, tree_calls, total_calls
    """
    analyze_calls = 0
    shell_calls = 0
    editor_calls = 0
    tree_calls = 0

    for tool_name, count in tool_counts.items():
        base = tool_name.rsplit('__', 1)[-1] if '__' in tool_name else tool_name
        if 'analyze' in base.lower():
            analyze_calls += count
        elif base == 'shell':
            shell_calls += count
        elif base in ('write', 'edit', 'text_editor'):
            editor_calls += count
        elif base == 'tree':
            tree_calls += count

    total = analyze_calls + shell_calls + editor_calls + tree_calls

    return {
        'analyze_calls': analyze_calls,
        'shell_calls': shell_calls,
        'editor_calls': editor_calls,
        'tree_calls': tree_calls,
        'total_calls': total
    }


def main():
    parser = argparse.ArgumentParser(
        description='Extract session metrics from goose DB for v7 benchmark'
    )
    parser.add_argument(
        '--session-name',
        required=True,
        help='Session name (e.g., v7-benchmark-R01-B5)'
    )
    parser.add_argument(
        '--db-path',
        type=Path,
        default=Path.home() / '.local' / 'share' / 'goose' / 'sessions' / 'sessions.db',
        help='Path to goose sessions database'
    )
    
    args = parser.parse_args()
    
    if not args.db_path.exists():
        print(json.dumps({"error": f"Database not found at {args.db_path}"}))
        exit(1)
    
    conn = get_db_connection(args.db_path)
    
    session_info = get_session_info(conn, args.session_name)
    if not session_info:
        print(json.dumps({"error": f"Session '{args.session_name}' not found"}))
        exit(1)
    
    session_id = session_info['id']
    messages = get_session_messages(conn, session_id)
    
    if not messages:
        print(json.dumps({"error": f"No messages found for session '{args.session_name}'"}))
        exit(1)
    
    # Extract metrics
    tool_counts = extract_tool_calls(messages)
    tool_categories = categorize_tool_calls(tool_counts)
    wall_time = calculate_wall_time(messages)
    parameter_usage = extract_parameter_usage(messages)
    
    result = {
        'session_name': args.session_name,
        'tokens': session_info['total_tokens'],
        'input_tokens': session_info['input_tokens'],
        'output_tokens': session_info['output_tokens'],
        'accumulated_total_tokens': session_info['accumulated_total_tokens'],
        'accumulated_input_tokens': session_info['accumulated_input_tokens'],
        'accumulated_output_tokens': session_info['accumulated_output_tokens'],
        'wall_seconds': wall_time,
        'tool_calls_detail': tool_counts,
        'analyze_calls': tool_categories['analyze_calls'],
        'shell_calls': tool_categories['shell_calls'],
        'editor_calls': tool_categories['editor_calls'],
        'total_calls': tool_categories['total_calls'],
        'parameter_usage': parameter_usage
    }
    
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
