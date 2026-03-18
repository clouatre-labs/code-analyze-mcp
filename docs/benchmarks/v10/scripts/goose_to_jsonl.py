#!/usr/bin/env python3
"""
goose_to_jsonl.py: Convert a goose session from SQLite to Claude-Code-compatible JSONL.

Goose stores sessions in ~/.local/share/goose/sessions/sessions.db.
collect.py and validate.py expect Claude Code JSONL format.

This converter:
1. Reads messages for a session from the goose SQLite DB
2. Maps goose toolRequest blocks -> Claude Code tool_use blocks
3. Injects session-level token totals into the first assistant message usage
4. Writes one JSON line per message in Claude-Code-compatible format

Usage:
    python3 goose_to_jsonl.py --session-id SESSION_ID --output OUTPUT.jsonl
    python3 goose_to_jsonl.py --session-id SESSION_ID --output OUTPUT.jsonl --db PATH_TO_DB
"""

import argparse
import json
import sqlite3
import sys
from pathlib import Path

DEFAULT_DB = Path.home() / ".local/share/goose/sessions/sessions.db"


def load_messages(db_path: Path, session_id: str):
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    try:
        rows = conn.execute(
            "SELECT role, content_json, created_timestamp FROM messages "
            "WHERE session_id = ? ORDER BY id ASC",
            (session_id,),
        ).fetchall()
    finally:
        conn.close()
    return rows


def load_session_tokens(db_path: Path, session_id: str):
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    try:
        row = conn.execute(
            "SELECT input_tokens, output_tokens FROM sessions WHERE id = ?",
            (session_id,),
        ).fetchone()
    finally:
        conn.close()
    if row:
        return row["input_tokens"] or 0, row["output_tokens"] or 0
    return 0, 0


def convert_content(blocks: list) -> list:
    """Convert goose content blocks to Claude-Code-compatible content blocks."""
    result = []
    for block in blocks:
        btype = block.get("type")
        if btype == "text":
            result.append({"type": "text", "text": block.get("text", "")})
        elif btype == "toolRequest":
            tc = block.get("toolCall", {})
            val = tc.get("value", {})
            name = val.get("name", "unknown")
            arguments = val.get("arguments", {})
            result.append({
                "type": "tool_use",
                "id": block.get("id", ""),
                "name": name,
                "input": arguments,
            })
        elif btype == "toolResponse":
            # Tool results are in user messages; map to tool_result for completeness
            tr = block.get("toolResult", {})
            val = tr.get("value", {})
            content_blocks = val.get("content", []) if isinstance(val, dict) else []
            text = ""
            if isinstance(content_blocks, list):
                text = " ".join(
                    b.get("text", "") for b in content_blocks if isinstance(b, dict)
                )
            elif isinstance(val, str):
                text = val
            result.append({
                "type": "tool_result",
                "tool_use_id": block.get("id", ""),
                "content": text,
            })
        # Skip unknown block types
    return result


def convert(db_path: Path, session_id: str, output_path: Path):
    rows = load_messages(db_path, session_id)
    if not rows:
        print(f"ERROR: No messages found for session {session_id}", file=sys.stderr)
        sys.exit(1)

    input_tokens, output_tokens = load_session_tokens(db_path, session_id)

    token_injected = False
    with open(output_path, "w") as out:
        for row in rows:
            role = row["role"]
            ts = row["created_timestamp"]
            try:
                blocks = json.loads(row["content_json"])
            except (json.JSONDecodeError, TypeError):
                blocks = []

            converted = convert_content(blocks)

            if role == "assistant":
                msg: dict = {"role": "assistant", "content": converted}
                # Inject session-level token totals into first assistant message
                if not token_injected:
                    msg["usage"] = {
                        "input_tokens": input_tokens,
                        "output_tokens": output_tokens,
                        "cache_creation_input_tokens": 0,
                        "cache_read_input_tokens": 0,
                    }
                    token_injected = True
                entry = {
                    "type": "assistant",
                    "message": msg,
                    "timestamp": ts,
                }
            else:
                entry = {
                    "type": role,
                    "message": {"role": role, "content": converted},
                    "timestamp": ts,
                }

            out.write(json.dumps(entry) + "\n")

    print(f"Wrote {len(rows)} messages to {output_path}")


def main():
    parser = argparse.ArgumentParser(description="Convert goose session to Claude-Code JSONL")
    parser.add_argument("--session-id", required=True, help="Goose session ID")
    parser.add_argument("--output", required=True, type=Path, help="Output JSONL path")
    parser.add_argument("--db", type=Path, default=DEFAULT_DB, help="Path to goose sessions.db")
    args = parser.parse_args()

    if not args.db.exists():
        print(f"ERROR: DB not found: {args.db}", file=sys.stderr)
        sys.exit(1)

    convert(args.db, args.session_id, args.output)


if __name__ == "__main__":
    main()
