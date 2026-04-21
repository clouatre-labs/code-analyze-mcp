#!/usr/bin/env python3
"""
Cross-client compatibility tests for aptu-coder.

Tests three MCP client implementations:
1. Raw stdio (JSON-RPC over newline-delimited JSON)
2. MCP Inspector CLI (npx @modelcontextprotocol/inspector)
3. Goose CLI (goose run --with-extension)

Each test class gracefully skips if the required client is unavailable.
"""

import json
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path


class McpStdioClient:
    """Helper for communicating with MCP server over raw stdio."""

    def __init__(self, server_path, timeout=30):
        """
        Spawn server and prepare for JSON-RPC communication.

        Args:
            server_path: Path to the server binary
            timeout: Timeout in seconds for each operation
        """
        self.server_path = Path(server_path)
        self.timeout = timeout
        self.proc = None
        self.msg_id = 0

    def start(self):
        """Start the server subprocess."""
        if not self.server_path.exists():
            raise FileNotFoundError(f"Server binary not found: {self.server_path}")
        self.proc = subprocess.Popen(
            [str(self.server_path)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

    def stop(self):
        """Terminate the server process gracefully."""
        if self.proc:
            try:
                self.proc.terminate()
                self.proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()

    def send_message(self, method, params=None, is_notification=False):
        """
        Send a JSON-RPC message to the server.

        Args:
            method: The RPC method name
            params: Optional method parameters
            is_notification: If True, send as notification (no id)

        Returns:
            The message ID used (0 for notifications)
        """
        msg = {
            "jsonrpc": "2.0",
            "method": method,
        }
        if not is_notification:
            self.msg_id += 1
            msg["id"] = self.msg_id
        if params:
            msg["params"] = params

        line = json.dumps(msg) + "\n"
        self.proc.stdin.write(line)
        self.proc.stdin.flush()

        return self.msg_id if not is_notification else 0

    def read_response(self, expected_id):
        """
        Read and parse the next JSON-RPC response.

        Args:
            expected_id: The message ID to expect

        Returns:
            The full JSON-RPC response object

        Raises:
            TimeoutError: If no response within timeout
            ValueError: If response format is invalid
        """
        start_time = time.time()
        while time.time() - start_time < self.timeout:
            try:
                line = self.proc.stdout.readline()
                if not line:
                    raise TimeoutError(f"No response after {self.timeout}s")
                response = json.loads(line)
                if response.get("id") == expected_id:
                    return response
            except json.JSONDecodeError as e:
                raise ValueError(f"Invalid JSON response: {e}")
            except Exception as e:
                raise TimeoutError(f"Error reading response: {e}")

        raise TimeoutError(f"No response with id {expected_id} after {self.timeout}s")


class TestRawStdio(unittest.TestCase):
    """Test the MCP server using raw stdio JSON-RPC."""

    @classmethod
    def setUpClass(cls):
        """Build the server binary before running tests."""
        repo_root = Path(__file__).parent.parent
        server_binary = repo_root / "target" / "release" / "aptu-coder"
        if not server_binary.exists():
            raise FileNotFoundError(f"Server binary not found: {server_binary}")
        cls.server_binary = server_binary

    def setUp(self):
        """Create a new client for each test."""
        self.client = McpStdioClient(self.server_binary)
        self.client.start()

    def tearDown(self):
        """Stop the client after each test."""
        self.client.stop()

    def _init_client(self):
        """Initialize client and send initialized notification."""
        init_id = self.client.send_message(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0"},
            },
        )
        self.client.read_response(init_id)
        self.client.send_message("notifications/initialized", is_notification=True)

    def test_initialize(self):
        """Happy path: Initialize the server and validate response format."""
        msg_id = self.client.send_message(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0"},
            },
        )
        response = self.client.read_response(msg_id)
        self.assertIn("result", response)
        result = response["result"]
        self.assertEqual(result["protocolVersion"], "2025-06-18")
        self.assertIn("capabilities", result)
        self.assertIn("serverInfo", result)
        server_info = result["serverInfo"]
        self.assertEqual(server_info["name"], "aptu-coder")
        self.assertIn("version", server_info)
        self.client.send_message("notifications/initialized", is_notification=True)

    def test_tools_list(self):
        """Happy path: List available tools and validate schema."""
        self._init_client()
        list_id = self.client.send_message("tools/list")
        response = self.client.read_response(list_id)
        self.assertIn("result", response)
        result = response["result"]
        self.assertIn("tools", result)
        self.assertGreater(len(result["tools"]), 0)
        analyze_tool = next(
            (t for t in result["tools"] if t["name"] == "analyze"), None
        )
        self.assertIsNotNone(analyze_tool, "analyze tool not found")
        self.assertIn("description", analyze_tool)
        schema = analyze_tool["inputSchema"]
        self.assertEqual(schema["type"], "object")
        self.assertIn("path", schema["properties"])

    def test_analyze_overview(self):
        """Happy path: Analyze a directory in overview mode."""
        self._init_client()
        repo_root = Path(__file__).parent.parent
        analyze_id = self.client.send_message(
            "tools/call",
            {
                "name": "analyze",
                "arguments": {"path": str(repo_root / "crates" / "aptu-coder" / "src")},
            },
        )
        response = self.client.read_response(analyze_id)
        self.assertIn("result", response)
        result = response["result"]
        self.assertIn("content", result)
        self.assertGreater(len(result["content"]), 0)
        content = result["content"][0]
        self.assertEqual(content["type"], "text")
        self.assertIn("PATH", content["text"])

    def test_analyze_file_details(self):
        """Happy path: Analyze a single file for details."""
        self._init_client()
        repo_root = Path(__file__).parent.parent
        src_main = repo_root / "crates" / "aptu-coder" / "src" / "main.rs"
        analyze_id = self.client.send_message(
            "tools/call",
            {
                "name": "analyze",
                "arguments": {"path": str(src_main)},
            },
        )
        response = self.client.read_response(analyze_id)
        self.assertIn("result", response)
        content = response["result"]["content"][0]
        output_text = content["text"]
        self.assertIn("FILE:", output_text)
        self.assertIn("F:", output_text)

    def test_analyze_error_invalid_path(self):
        """Edge case: Error handling for nonexistent path."""
        self._init_client()
        analyze_id = self.client.send_message(
            "tools/call",
            {
                "name": "analyze",
                "arguments": {"path": "/nonexistent/path/to/file"},
            },
        )
        response = self.client.read_response(analyze_id)
        if "error" in response:
            self.assertIn("code", response["error"])
            self.assertIn("message", response["error"])
        else:
            self.assertIn("result", response)


@unittest.skipUnless(shutil.which("npx"), "npx not available (install Node.js)")
class TestMcpInspector(unittest.TestCase):
    """Test the MCP server using MCP Inspector CLI."""

    @classmethod
    def setUpClass(cls):
        """Verify server binary exists."""
        repo_root = Path(__file__).parent.parent
        cls.server_binary = repo_root / "target" / "release" / "aptu-coder"
        if not cls.server_binary.exists():
            raise FileNotFoundError(f"Server binary not found: {cls.server_binary}")
        cls.repo_root = repo_root

    def setUp(self):
        """Create a temporary config file for Inspector."""
        self.config_file = tempfile.NamedTemporaryFile(
            mode="w",
            suffix=".json",
            delete=False,
        )
        config = {
            "mcpServers": {
                "code-analyze": {
                    "command": str(self.server_binary),
                    "args": [],
                }
            }
        }
        json.dump(config, self.config_file)
        self.config_file.close()

    def tearDown(self):
        """Clean up the temporary config file."""
        Path(self.config_file.name).unlink(missing_ok=True)

    def _run_inspector(self, args):
        """Run inspector with given args and return parsed JSON response."""
        result = subprocess.run(
            ["npx", "--yes", "@modelcontextprotocol/inspector", "--cli"] + args,
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        try:
            return json.loads(result.stdout.strip())
        except json.JSONDecodeError:
            self.fail(f"Invalid JSON output: {result.stdout.strip()}")

    def test_inspector_tools_list(self):
        """Happy path: Inspector lists tools correctly."""
        response = self._run_inspector(
            [
                "--config",
                self.config_file.name,
                "--server",
                "code-analyze",
                "--method",
                "tools/list",
            ]
        )
        self.assertIn("tools", response)
        self.assertGreater(len(response["tools"]), 0)
        analyze_tool = next(
            (t for t in response["tools"] if t["name"] == "analyze"), None
        )
        self.assertIsNotNone(analyze_tool)
        self.assertIn("description", analyze_tool)

    def test_inspector_analyze_file(self):
        """Happy path: Inspector can call analyze on a file."""
        src_main = self.repo_root / "crates" / "aptu-coder" / "src" / "main.rs"
        response = self._run_inspector(
            [
                "--config",
                self.config_file.name,
                "--server",
                "code-analyze",
                "--method",
                "tools/call",
                "--tool-name",
                "analyze",
                "--tool-arg",
                f"path={src_main}",
            ]
        )
        self.assertIn("content", response)
        self.assertGreater(len(response["content"]), 0)
        content = response["content"][0]
        self.assertEqual(content["type"], "text")
        self.assertIn("text", content)


@unittest.skipUnless(shutil.which("goose"), "goose not available")
class TestGooseCli(unittest.TestCase):
    """Test the MCP server using Goose CLI."""

    @classmethod
    def setUpClass(cls):
        """Verify server binary exists."""
        repo_root = Path(__file__).parent.parent
        cls.server_binary = repo_root / "target" / "release" / "aptu-coder"
        if not cls.server_binary.exists():
            raise FileNotFoundError(f"Server binary not found: {cls.server_binary}")
        cls.repo_root = repo_root

    def _run_goose(self, task):
        """Run goose with given task and return parsed JSON response."""
        result = subprocess.run(
            [
                "goose",
                "run",
                "--with-extension",
                str(self.server_binary),
                "--no-profile",
                "--quiet",
                "--no-session",
                "--output-format",
                "json",
                "-t",
                task,
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        try:
            return json.loads(result.stdout.strip())
        except json.JSONDecodeError:
            self.fail(f"Invalid JSON output: {result.stdout.strip()}")

    def test_goose_tool_discovery(self):
        """Happy path: Goose discovers the analyze tool."""
        response = self._run_goose("List available tools")
        self.assertIn("messages", response)
        self.assertGreater(len(response["messages"]), 0)
        found_analyze = any(
            "analyze" in str(msg.get("content", "")).lower()
            for msg in response["messages"]
        )
        self.assertTrue(found_analyze, "goose should discover analyze tool")

    def test_goose_analyze_file(self):
        """Happy path: Goose can call analyze on a file."""
        src_main = self.repo_root / "crates" / "aptu-coder" / "src" / "main.rs"
        response = self._run_goose(
            f"Analyze the file {src_main} and describe its structure"
        )
        self.assertIn("messages", response)
        self.assertGreater(len(response["messages"]), 0)


if __name__ == "__main__":
    unittest.main()
