# v10 Benchmark Execution Prompt

Use this document to run the v10 benchmark end-to-end.
Conditions A, A2, B, C use claude CLI. Conditions D and E use a goose session.

---

## Prerequisites

```bash
# 1. Clone and pin Django
mkdir -p /tmp/benchmark-repos
git clone https://github.com/django/django /tmp/benchmark-repos/django
git -C /tmp/benchmark-repos/django checkout 6b90f8a8d6994dc62cd91dde911fe56ec3389494

# 2. Verify the MCP server is installed and on PATH
code-analyze-mcp --version   # or: cargo install --path . --profile release

# 3. For conditions D and E
export OPENROUTER_API_KEY=<your-key>

# 4. Create results directory
mkdir -p docs/benchmarks/v10/results/runs
```

---

## Run order

Execute runs in this exact order (seed=42 randomization, blinding maintained):

| # | RUN_ID | Condition | Model | Runner |
|---|--------|-----------|-------|--------|
| 1 | R01 | C4 | global.anthropic.claude-sonnet-4-6 | claude CLI |
| 2 | R02 | A2_2 | global.anthropic.claude-haiku-4-5-20251001-v1:0 | claude CLI |
| 3 | R03 | D3 | minimax/minimax-m2.5 | goose |
| 4 | R04 | E4 | mistralai/mistral-small-2603 | goose |
| 5 | R05 | C2 | global.anthropic.claude-sonnet-4-6 | claude CLI |
| 6 | R06 | B3 | global.anthropic.claude-haiku-4-5-20251001-v1:0 | claude CLI |
| 7 | R07 | D2 | minimax/minimax-m2.5 | goose |
| 8 | R08 | B4 | global.anthropic.claude-haiku-4-5-20251001-v1:0 | claude CLI |
| 9 | R09 | C3 | global.anthropic.claude-sonnet-4-6 | claude CLI |
| 10 | R10 | D1 | minimax/minimax-m2.5 | goose |
| 11 | R11 | A2 | global.anthropic.claude-sonnet-4-6 | claude CLI |
| 12 | R12 | C1 | global.anthropic.claude-sonnet-4-6 | claude CLI |
| 13 | R13 | E2 | mistralai/mistral-small-2603 | goose |
| 14 | R14 | A2_3 | global.anthropic.claude-haiku-4-5-20251001-v1:0 | claude CLI |
| 15 | R15 | B2 | global.anthropic.claude-haiku-4-5-20251001-v1:0 | claude CLI |
| 16 | R16 | A3 | global.anthropic.claude-sonnet-4-6 | claude CLI |
| 17 | R17 | E3 | mistralai/mistral-small-2603 | goose |
| 18 | R18 | A2_1 | global.anthropic.claude-haiku-4-5-20251001-v1:0 | claude CLI |
| 19 | R19 | D4 | minimax/minimax-m2.5 | goose |
| 20 | R20 | A2_4 | global.anthropic.claude-haiku-4-5-20251001-v1:0 | claude CLI |
| 21 | R21 | B1 | global.anthropic.claude-haiku-4-5-20251001-v1:0 | claude CLI |
| 22 | R22 | A1 | global.anthropic.claude-sonnet-4-6 | claude CLI |
| 23 | R23 | A4 | global.anthropic.claude-sonnet-4-6 | claude CLI |
| 24 | R24 | E1 | mistralai/mistral-small-2603 | goose |

---

## Executing a run with the runner script

For conditions A, A2, B, C:

```bash
cd docs/benchmarks/v10
DISABLE_PROMPT_CACHING=1 ./run.sh <RUN_ID>
```

Example:

```bash
./run.sh R01   # C4: Sonnet + MCP
./run.sh R11   # A2: Sonnet + native
./run.sh R06   # B3: Haiku + MCP
```

The script:
1. Looks up the condition from the blinding map
2. Selects the system prompt and model
3. Runs `claude -p` with `DISABLE_PROMPT_CACHING=1`
4. Copies the session JSONL to `results/runs/<RUN_ID>-session.jsonl`
5. Validates the output JSON

---

## Executing a goose run (conditions D and E)

Conditions D and E require a goose session with the code-analyze extension loaded.
The runner script emits the exact instructions; run it first to see them:

```bash
./run.sh R03   # prints goose session instructions for D3
```

Then start the session manually:

```bash
OPENROUTER_API_KEY=$OPENROUTER_API_KEY \
goose session \
  --provider openrouter \
  --model minimax/minimax-m2.5 \
  --with-extension code-analyze
```

Inside the session, paste the contents of the appropriate system prompt file followed
by the task file:

- System prompt: `prompts/condition-d-treatment-minimax.md` (or `condition-e-treatment-mistral.md`)
- Replace `TARGET_REPO_PATH` with `/tmp/benchmark-repos/django`
- Replace `OUTPUT_PATH` with the absolute path to `results/runs/<RUN_ID>-report.json`
- Replace `RUN_ID` with the actual run ID (e.g. `R03`)
- Paste `prompts/task.md` as the user message

After the session completes:
1. Save the goose session JSONL to `results/runs/<RUN_ID>-session.jsonl`
2. Verify the report JSON was written to `results/runs/<RUN_ID>-report.json`

---

## After each run: collect metrics

```bash
cd docs/benchmarks/v10

# Conditions A, A2, B, C (GCP):
python scripts/collect.py \
  --session-file results/runs/<RUN_ID>-session.jsonl \
  --output-file results/runs/<RUN_ID>-report.json \
  --model <MODEL> \
  --provider gcp_vertex_ai \
  --model-id <MODEL_ID> \
  > results/runs/<RUN_ID>-metrics.json

# Example for R01 (C4, Sonnet):
python scripts/collect.py \
  --session-file results/runs/R01-session.jsonl \
  --output-file results/runs/R01-report.json \
  --model sonnet \
  --provider gcp_vertex_ai \
  --model-id global.anthropic.claude-sonnet-4-6 \
  > results/runs/R01-metrics.json

# Conditions D (MiniMax):
python scripts/collect.py \
  --session-file results/runs/<RUN_ID>-session.jsonl \
  --output-file results/runs/<RUN_ID>-report.json \
  --model minimax \
  --provider openrouter \
  --model-id minimax/minimax-m2.5 \
  > results/runs/<RUN_ID>-metrics.json

# Conditions E (Mistral):
python scripts/collect.py \
  --session-file results/runs/<RUN_ID>-session.jsonl \
  --output-file results/runs/<RUN_ID>-report.json \
  --model mistral \
  --provider openrouter \
  --model-id mistralai/mistral-small-2603 \
  > results/runs/<RUN_ID>-metrics.json
```

---

## After each run: validate tool isolation

```bash
# Map RUN_ID to condition letter using the blinding map in run-order.txt, then:
python scripts/validate.py \
  --session-file results/runs/<RUN_ID>-session.jsonl \
  --condition <A|A2|B|C|D|E>

# Example:
python scripts/validate.py \
  --session-file results/runs/R01-session.jsonl \
  --condition C
```

Exit 0 = PASS. Exit 1 = FAIL (record violation in scores-template.json notes).

---

## Pinning the commit SHA

The commit SHA is already pinned in `run-order.txt` (`6b90f8a8d6994dc62cd91dde911fe56ec3389494`). If re-pinning after a fresh clone, replace the existing SHA:

```bash
COMMIT=$(git -C /tmp/benchmark-repos/django rev-parse HEAD)
sed -i '' "s/pinned_commit: [0-9a-f]*/pinned_commit: $COMMIT/" docs/benchmarks/v10/run-order.txt
```

---

## After all 24 runs: blind scoring

1. Use only `R01`–`R24` labels when reviewing outputs — do not consult the blinding map yet
2. For each run, open `results/runs/<RUN_ID>-report.json`
3. Score 4 dimensions (0–3 each) using the rubric in `methodology.md`
4. Fill `scores-template.json` `per_run_scores` using the run label (A1, B3, C4, etc.)
   — the blinding map in `scores-template.json` translates R-IDs to condition+rep
5. After all 24 scores are entered, run the analysis

---

## Analysis

```bash
cd docs/benchmarks/v10
python scripts/analyze.py --scores-file scores-template.json
```

Outputs Markdown tables to stdout:
- Quality analysis (medians + pairwise tests)
- 15 Mann-Whitney U tests (Bonferroni α=0.0033)
- 2×2 factorial summary
- Tool call breakdown
- Cost analysis (effective_cost_per_qp with reliability)
- Protocol violations
- Cache and token metrics

---

## Condition–model–prompt reference

| Condition | Model | System prompt file | Disallowed tools |
|-----------|-------|--------------------|-----------------|
| A | global.anthropic.claude-sonnet-4-6 | condition-a-control.md | analyze_directory, analyze_file, analyze_symbol |
| A2 | global.anthropic.claude-haiku-4-5-20251001-v1:0 | condition-a2-haiku-native.md | analyze_directory, analyze_file, analyze_symbol |
| B | global.anthropic.claude-haiku-4-5-20251001-v1:0 | condition-b-treatment-haiku.md | Glob, Grep, Read, Bash |
| C | global.anthropic.claude-sonnet-4-6 | condition-c-treatment-sonnet.md | Glob, Grep, Read, Bash |
| D | minimax/minimax-m2.5 | condition-d-treatment-minimax.md | Glob, Grep, Read, Bash |
| E | mistralai/mistral-small-2603 | condition-e-treatment-mistral.md | Glob, Grep, Read, Bash |
