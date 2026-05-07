#!/usr/bin/env python3
"""
Thin asyncio proxy for aptu-coder MCP server.

Spawns aptu-coder subprocess and intercepts JSON-RPC messages.
Injects _meta with profile information into the first initialize message,
then forwards all messages unchanged in both directions.

Environment variables:
  APTU_CODER_PROFILE -- profile name to inject (e.g., "edit")
                        If unset, no injection occurs.
"""

import asyncio
import json
import os
import sys


async def main():
    """Main proxy loop."""
    # Spawn aptu-coder subprocess
    proc = await asyncio.create_subprocess_exec(
        "aptu-coder",
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )

    # Get profile from environment
    profile = os.environ.get("APTU_CODER_PROFILE")

    # Track whether we've seen the initialize message
    initialize_seen = False

    async def forward_stdin_to_subprocess():
        """Read from stdin, inject _meta into initialize, forward to subprocess."""
        nonlocal initialize_seen
        try:
            loop = asyncio.get_event_loop()
            while True:
                # Read a line from stdin (blocking, but in executor)
                line = await loop.run_in_executor(None, sys.stdin.readline)
                if not line:
                    # EOF
                    proc.stdin.close()
                    break

                line = line.rstrip("\n")
                if not line:
                    continue

                try:
                    msg = json.loads(line)
                except json.JSONDecodeError:
                    # Not JSON, pass through
                    proc.stdin.write((line + "\n").encode())
                    await proc.stdin.drain()
                    continue

                # Check if this is the initialize message
                if (
                    not initialize_seen
                    and isinstance(msg, dict)
                    and msg.get("method") == "initialize"
                ):
                    initialize_seen = True
                    if profile:
                        # Inject _meta
                        if "_meta" not in msg:
                            msg["_meta"] = {}
                        msg["_meta"]["io.clouatre-labs/profile"] = profile
                    line = json.dumps(msg)

                # Forward to subprocess
                proc.stdin.write((line + "\n").encode())
                await proc.stdin.drain()
        except Exception as e:
            print(f"Error in stdin forwarder: {e}", file=sys.stderr)
            proc.stdin.close()

    async def forward_stdout_from_subprocess():
        """Read from subprocess stdout, forward to stdout."""
        try:
            while True:
                line = await proc.stdout.readline()
                if not line:
                    break
                sys.stdout.buffer.write(line)
                sys.stdout.buffer.flush()
        except Exception as e:
            print(f"Error in stdout forwarder: {e}", file=sys.stderr)

    async def forward_stderr_from_subprocess():
        """Read from subprocess stderr, forward to stderr."""
        try:
            while True:
                line = await proc.stderr.readline()
                if not line:
                    break
                sys.stderr.buffer.write(line)
                sys.stderr.buffer.flush()
        except Exception as e:
            print(f"Error in stderr forwarder: {e}", file=sys.stderr)

    # Run all forwarding tasks concurrently
    try:
        await asyncio.gather(
            forward_stdin_to_subprocess(),
            forward_stdout_from_subprocess(),
            forward_stderr_from_subprocess(),
        )
    except KeyboardInterrupt:
        pass
    finally:
        # Wait for subprocess to finish
        await proc.wait()
        sys.exit(proc.returncode or 0)


if __name__ == "__main__":
    asyncio.run(main())
