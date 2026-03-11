# v7 Benchmark Methodology

## Research Question

Can agents discover and use new tool parameters to optimize code analysis token efficiency without sacrificing quality?

## Hypothesis

**Primary Hypothesis:** When tool parameters (summary, cursor, page_size) are documented in the tool description, agents will use them in >40% of analyze calls and achieve lower token consumption than agents without parameter knowledge.

**Secondary Hypothesis:** Parameter usage will not introduce quality regressions; v7B quality will be equivalent to or better than v6B.

## Experimental Design

### Conditions

- **Condition A (Control):** code-analyze-mcp__analyze tool with baseline documentation (no parameter docs). Represents agent without parameter knowledge.
- **Condition B (Treatment):** code-analyze-mcp__analyze tool with extended documentation including summary, cursor, and page_size parameter descriptions and examples. Represents agent with parameter knowledge.

Both conditions perform the same task: cross-module analysis on lsd codebase.

### Participants

10 runs total, 5 per condition, randomized order (seed=256):
1. B5, 2. B4, 3. A2, 4. B1, 5. A1, 6. A4, 7. A3, 8. B2, 9. A5, 10. B3

Model: claude-haiku-4-5, temperature=0.5 (consistent with v6).

## Instrumentation

### Parameter Usage Tracking

Scripts track three parameters during execution:

1. **summary:** Boolean flag to collapse output to top-level summary
   - Detected in tool call inputs: input.get('summary') is True
   - Usefulness: Reduces token consumption for large files/codebases (>50K chars)

2. **cursor:** String token for paginated result navigation
   - Detected in tool call inputs: 'cursor' in input and input['cursor'] is not None/empty
   - Usefulness: Enables browsing large result sets without re-fetching
   - pagination_used flag: True if cursor_calls > 0

3. **page_size:** Integer to limit output size (default: 50000)
   - Detected in tool call inputs: 'page_size' in input and input['page_size'] is not None
   - Usefulness: Fine-tune response length for token efficiency

### Extraction Logic

**collect.py:**
- Iterates tool_use blocks from assistant messages
- For each code-analyze-mcp__analyze call, inspects input dict:
  - summary_count: count calls with summary=True
  - cursor_calls: count calls with non-empty cursor
  - page_size_overrides: count calls with non-null page_size
  - pagination_used: True if cursor_calls > 0
- Output in per_run_scores[run_id].parameter_usage dict

**validate.py:**
- Parses tool_use blocks and extracts parameter usage same as collect.py
- Prints parameter_usage tracking details for validation
- Used pre-analysis to confirm agents actually attempted parameter usage

### Metrics

**Token Efficiency:**
- Total tokens, input tokens, output tokens from session
- Wall time (last message - first message)
- Tool call counts (analyze_calls, shell_calls, editor_calls)

**Parameter Adoption:**
- summary_count, cursor_calls, page_size_overrides per run
- Aggregated: % of B runs using each parameter

**Quality:**
- Rubric scores (0-3) on four dimensions:
  - structural_accuracy: Module identification and relationships
  - cross_module_tracing: Call chain and data flow tracing
  - approach_quality: Change proposal reasoning
  - tool_efficiency: Tool call efficiency (5 or fewer = 3 points)
- Total score per run: sum of four dimensions (0-12)

## Rubric

### Structural Accuracy (0-3)

- **3 (Full):** Correctly identifies all major modules (flags, meta, theme, root) and submodules; responsibilities clearly defined; key types (Meta, Block, Color, Icon) accurately described
- **2 (Partial):** Identifies most modules and responsibilities; minor omissions or slight generalization
- **1 (Minimal):** Partial module coverage; significant gaps in understanding relationships
- **0 (None):** Major components missing; fundamental misunderstanding

### Cross-Module Tracing (0-3)

- **3 (Full):** Complete trace from Meta::from_path through sorting, icon resolution, color resolution, to display output; intermediate types (Meta, Vec<Metadata>, Block) documented at each stage
- **2 (Partial):** Key stages traced; minor gaps or simplified type flow
- **1 (Minimal):** Partial trace; missing intermediate steps or type information
- **0 (None):** No meaningful data flow or entirely incorrect

### Approach Quality (0-3)

- **3 (Full):** Change proposal identifies all affected files (display.rs, core.rs, new hash module); new types (HashBlock struct); patterns to follow (existing Block/Icon pattern); integration point (render pipeline); and realistic risks (hash computation cost, dependency additions)
- **2 (Partial):** Reasonable proposal; most files and types identified; risks partially addressed
- **1 (Minimal):** Incomplete proposal; missing files, types, or risk analysis
- **0 (None):** Superficial or incorrect proposal; major gaps

### Tool Efficiency (0-3)

- **3 (Full):** 5 or fewer analysis tool calls; focused exploration; clear synthesis path
- **2 (Partial):** 6-10 tool calls; somewhat exploratory but reaches conclusions
- **1 (Minimal):** 11-20 tool calls; extensive exploration; delayed synthesis
- **0 (None):** >20 tool calls; inefficient; redundant exploration

## Statistical Test

**Mann-Whitney U Test** (non-parametric; comparing two independent groups):
- Null hypothesis: Distribution of scores is the same for Condition A and B
- Alternative: Distributions differ
- Test on total scores and each dimension separately
- Report U-statistic, z-score, p-value, rank-biserial r (effect size)
- Interpretation: |r| >= 0.3 (small), >= 0.5 (medium), >= 0.7 (large effect)

## Outcome Definitions

### Success (Primary)

v7B parameter usage meets adoption goal and token efficiency improves:
- >40% of v7B runs use new parameters (summary, cursor, or page_size in >0 calls)
- v7B median total tokens <= v6B median - 5% (reduction of 5%+ from baseline)
- v7B quality (median total score) >= v6B quality (no regression)

### Partial Success

Parameters are used but efficiency gain is modest:
- 20-40% parameter adoption in v7B
- v7B tokens within 5% of v6B
- v7B quality maintained

### Failure

Agents do not discover or apply parameters effectively:
- <20% parameter adoption in v7B
- v7B tokens > v6B or increased
- Quality regression in v7B

## Blinding

Randomized order with seed=256 ensures runs are analyzed in neutral sequence. Mapping in scores-template.json tracks real condition (A/B) but scorers blind during evaluation; mapping revealed only after all scores submitted.

Mapping (seed=256):
- R01=B5, R02=B4, R03=A2, R04=B1, R05=A1, R06=A4, R07=A3, R08=B2, R09=A5, R10=B3

## Reproducibility

All parameters fixed:
- Model: claude-haiku-4-5
- Temperature: 0.5
- Target repo: lsd-rs/lsd (fixed at v6 commit)
- Task description: Verbatim from v6 (task.md)
- Tool isolation: Validated before analysis (validate.py)
- Random seed for run order: 256
- Scoring rubric: Identical to v6 (no changes to dimensions or scales)
