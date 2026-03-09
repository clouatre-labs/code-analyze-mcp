#!/usr/bin/env python3
"""
validate.py: Tool isolation validation for v5 benchmark runs.

Queries goose sessions DB, extracts tool call information, and validates
tool isolation constraints:

- Condition A: developer__analyze used; no code-analyze-mcp calls
- Condition B: code-analyze-mcp__analyze used; no developer__analyze calls;
               no rg structural analysis patterns

Usage:
    python3 validate.py --session-name R01 --condition A
    python3 validate.py --session-name R01 --condition B --db-path ~/.local/share/goose/sessions/sessions.db

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
    
    Messages with role 'assistant' contain tool_use blocks:
    {
        "content": [
            {"type": "tool_use", "name": "tool_name", "input": {...}},
            ...
        ]
    }
    
    Returns list of dicts: [{"tool": "name", "input": {...}}, ...]
    """
    tools = []
    
    for msg in messages:
        if msg['role'] != 'assistant':
            continue
        
        content = msg.get('content', [])
        if isinstance(content, list):
            for block in content:
                if isinstance(block, dict) and block.get('type') == 'tool_use':
                    tools.append({
                        'tool': block.get('name', 'unknown'),
                        'input': block.get('input', {})
                    })
    
    return tools


def count_tool_calls(tools: List[Dict]) -> Dict[str, int]:
    """Count tool calls by name"""
    counts = {}
    for tool in tools:
        name = tool['tool']
        counts[name] = counts.get(name, 0) + 1
    return counts


def has_rg_structural_patterns(tools: List[Dict]) -> bool:
    """
    Check for rg structural analysis patterns in shell calls.
    
    Looks for rg commands with patterns like 'fn ', 'struct ', 'impl ', 'mod ', 'use '
    which indicate structural code search rather than generic file search.
    """
    structural_keywords = [r'\bfn\s', r'\bstruct\s', r'\bimpl\s', r'\bmod\s', r'\buse\s']
    
    for tool in tools:
        if 'developer__shell' in tool['tool'] or 'shell' in tool['tool'].lower():
            # Check the command
            cmd = tool.get('input', {}).get('command', '')
            if isinstance(cmd, str) and 'rg' in cmd:
                for pattern in structural_keywords:
                    if re.search(pattern, cmd):
                        return True
    
    return False


def validate_condition_a(tools_by_name: Dict[str, int], tools_list: List[Dict]) -> Tuple[bool, List[str]]:
    """
    Validate Condition A:
    - developer__analyze must be used
    - code-analyze-mcp__analyze must NOT be used
    """
    issues = []
    
    # Check for developer__analyze
    has_analyze = any('analyze' in name and 'code-analyze-mcp' not in name for name in tools_by_name)
    if not has_analyze:
        issues.append("ERROR: developer__analyze not used (required for Condition A)")
    
    # Check for code-analyze-mcp
    has_mcp = any('code-analyze-mcp' in name for name in tools_by_name)
    if has_mcp:
        issues.append(f"ERROR: code-analyze-mcp tool used (forbidden for Condition A): {[n for n in tools_by_name if 'code-analyze-mcp' in n]}")
    
    passed = len(issues) == 0
    return passed, issues


def validate_condition_b(tools_by_name: Dict[str, int], tools_list: List[Dict]) -> Tuple[bool, List[str]]:
    """
    Validate Condition B:
    - code-analyze-mcp__analyze must be used
    - developer__analyze must NOT be used
    - rg structural patterns must NOT be present
    """
    issues = []
    
    # Check for code-analyze-mcp
    has_mcp = any('code-analyze-mcp' in name for name in tools_by_name)
    if not has_mcp:
        issues.append("ERROR: code-analyze-mcp__analyze not used (required for Condition B)")
    
    # Check for developer__analyze
    has_analyze = any('developer__analyze' in name for name in tools_by_name)
    if has_analyze:
        issues.append("ERROR: developer__analyze used (forbidden for Condition B; native extension should be disabled)")
    
    # Check for rg structural patterns
    if has_rg_structural_patterns(tools_list):
        issues.append("WARNING: rg with structural patterns detected (should use code-analyze-mcp for structural analysis)")
    
    passed = len(issues) == 0
    return passed, issues


def main():
    parser = argparse.ArgumentParser(
        description='Validate tool isolation for a v5 benchmark run'
    )
    parser.add_argument(
        '--session-name',
        required=True,
        help='Session name (e.g., R01, R02)'
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
    else:  # B
        passed, issues = validate_condition_b(tools_by_name, tools)
    
    print(f"\nValidation: {'PASS' if passed else 'FAIL'}")
    
    if issues:
        for issue in issues:
            print(f"  {issue}")
    
    exit(0 if passed else 1)


if __name__ == '__main__':
    main()
