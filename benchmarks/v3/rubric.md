# Scoring Rubric and Blinding Procedure

## Scoring Dimensions

All responses are scored on four dimensions, each on a 0-3 scale.

### 1. Structural Accuracy (0-3)

Measures how accurately the response identifies the main modules and their roles in the data flow.

- **0:** Incorrect or missing module identification; major structural errors
- **1:** Identifies some modules but with significant gaps or misunderstandings
- **2:** Identifies most key modules correctly; minor gaps or inaccuracies
- **3:** Accurately identifies all key modules and their roles; matches ground truth

**Scoring Checklist:**
- Entry point identified (main.rs, argument parsing)
- File reading module identified
- Syntax highlighting module identified
- Output rendering module identified
- Exit point identified (stdout)
- Module roles are accurate

### 2. Cross-Module Tracing (0-3)

Measures how well the response traces data flow between modules and documents dependencies.

- **0:** No cross-module interactions identified or completely incorrect
- **1:** Identifies 1-2 interactions; many gaps or errors
- **2:** Identifies 3-4 interactions correctly; mostly accurate dependencies
- **3:** Identifies 5+ interactions correctly; accurate dependency graph

**Scoring Checklist:**
- At least 3 cross-module interactions documented
- Data flow direction is correct (input -> processing -> output)
- Dependencies are accurately represented
- Interaction descriptions are specific and clear

### 3. Approach Quality (0-3)

Measures the quality of the extension proposal and whether it identifies the correct abstraction point.

- **0:** Infeasible proposal; does not work or fundamentally misunderstands the codebase
- **1:** Works but naive; adds unnecessary abstraction or modifies core logic unnecessarily
- **2:** Good proposal with tradeoffs noted; identifies a valid extension point
- **3:** Elegant, minimal, safe; identifies the Printer trait as the extension point and proposes HtmlPrinter implementation

**Scoring Checklist:**
- Extension proposal is feasible and would actually work
- Proposal identifies an abstraction point (trait or interface)
- Proposal minimizes changes to existing code
- Proposal for HTML output format (not custom color themes)
- Identifies Printer trait as the extension point (score 3) vs other approaches (score 1-2)

### 4. Tool Efficiency (0-3)

Measures how effectively the analysis tool was used without redundant or unnecessary queries.

- **0:** Excessive tool use; many redundant queries or tool misuse
- **1:** Some inefficiency; more queries than necessary
- **2:** Mostly efficient; minimal redundancy
- **3:** Highly efficient; minimal queries, maximum information extraction

**Scoring Checklist:**
- No redundant queries (same module queried multiple times unnecessarily)
- Queries are well-targeted and specific
- Information from each query is fully utilized
- Tool parameters (max_depth, focus) are used appropriately

## Blinding Procedure

To prevent scorer bias, all runs are scored blind to their condition (Condition A vs B).

### Step 1: Prepare Blinded Responses
1. Extract all 10 run outputs from `results/run-{id}.json`
2. Create a mapping file (kept confidential): `run_id -> condition` (e.g., `A1 -> Condition A`)
3. Rename outputs to blinded IDs: `R1, R2, ..., R10` (randomized order)
4. Remove any condition labels or tool names from the response text
5. Store blinded responses in a temporary directory

### Step 2: Randomize Scoring Order
1. Generate a random permutation of `[R1, R2, ..., R10]`
2. Score in this randomized order to prevent fatigue bias
3. Do not score all Condition A runs consecutively

### Step 3: Score Blinded Responses
1. For each blinded response (R1-R10):
   - Score on all four dimensions (0-3 each)
   - Record scores in `scores.json` template
   - Add brief notes on scoring rationale
2. Do not attempt to identify the condition during scoring
3. If condition becomes obvious, note it but do not adjust scoring

### Step 4: Unblind and Analyze
1. After all 10 runs are scored, unblind using the confidential mapping
2. Separate scores by condition (A: R?, R?, ... vs B: R?, R?, ...)
3. Calculate per-condition statistics:
   - Median score per dimension
   - Range (min-max) per dimension
   - Total score (sum of 4 dimensions, max 12)
4. Perform Mann-Whitney U test on total scores

## Scoring Template

For each run, record:

```json
{
  "run_id": "R1",
  "structural_accuracy": 2,
  "cross_module_tracing": 2,
  "approach_quality": 3,
  "tool_efficiency": 2,
  "total_score": 9,
  "notes": "Clear module identification but missed some interactions. Good systematic approach."
}
```

## Inter-Rater Reliability

This benchmark uses a single scorer (no inter-rater reliability possible). To mitigate scorer bias:
- Blinding procedure (above) prevents condition bias
- Randomized scoring order prevents fatigue bias
- Detailed rubric ensures consistent application of criteria
- Notes document scoring rationale for auditability

## Decision Framework

After scoring and statistical analysis:

1. **If Mann-Whitney U p-value < 0.05:** Significant difference detected
   - Report effect size (rank-biserial correlation)
   - Determine which condition is superior
   - Conclude tool isolation is effective or ineffective

2. **If Mann-Whitney U p-value >= 0.05:** No significant difference
   - Report effect size (may still be meaningful)
   - Conclude tools are equivalent for this task
   - Note: small sample size (n=5) limits power

3. **Qualitative Assessment:**
   - Review median scores and ranges per condition
   - Identify which dimensions show largest differences
   - Document any patterns in tool usage or response quality
