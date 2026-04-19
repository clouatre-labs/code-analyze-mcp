# Scripts

This directory contains benchmarking and testing utilities for aptu-coder.

## Benchmarking

### bench-v12-run.sh, bench-v13-run.sh, bench-v14-run.sh

Benchmark runners for versions 12, 13, and 14 of the aptu-coder evaluation.

Each version supports parameterized conditions (A, B, C, D) to compare:
- Different models (Sonnet vs Haiku)
- Different tool modes (MCP vs native)

**Usage:**
```bash
./bench-v12-run.sh <CONDITION_ID> <RUN_ID>
```

Example:
```bash
./bench-v12-run.sh A A-pilot-1
```

## Testing

### cross-client-compat.py

Python script that validates cross-client compatibility and tool isolation.
Ensures the MCP server behaves consistently across different client implementations.
