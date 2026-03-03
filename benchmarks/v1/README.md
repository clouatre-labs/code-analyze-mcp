# Benchmark v1: Proof of Concept (Flawed)

This directory contains the original v1 benchmark, preserved for reference only.

## Status

**FLAWED - Do not use for conclusions.** See [issue #60](https://github.com/clouatre-labs/code-analyze-mcp/issues/60) for analysis.

## Why v1 Was Flawed

1. **No repetitions:** Single run per condition (A1, B1 only). Cannot compute means, confidence intervals, or detect variance.
2. **Insufficient for comparison:** One data point per condition is anecdotal, not statistical.
3. **No reproducibility metadata:** Session IDs and exact conditions were not recorded.

## What v1 Tested

- **Target repo:** sharkdp/bat (10.8K LOC Rust, 40 files)
- **Task:** Map data flow from user input to terminal output
- **Conditions:**
  - Control (A): `developer` extension only (no `developer__analyze`)
  - Treatment (B): `developer` + `code-analyze-mcp`
- **Model:** claude-haiku-4-5@20251001

## Files

- `run-a-control.json` - Raw session data from control run
- `run-b-treatment.json` - Raw session data from treatment run
- `conditions.json` - Metadata about the benchmark setup

## Use v2 Instead

See `../v2/` for the properly designed benchmark with 3 repetitions per condition, full reproducibility metadata, and statistical analysis.
