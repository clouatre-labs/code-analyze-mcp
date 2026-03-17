#!/usr/bin/env python3
"""
analyze.py: Statistical analysis for v10 benchmark scores (6 conditions).

Reads a filled scores.json and computes:
- Per-condition medians and ranges for quality dimensions
- 15 pairwise Mann-Whitney U tests (C(6,2)=15) with Bonferroni correction
- Rank-biserial r (effect size)
- 2x2 factorial summary table (rows: models, cols: native/MCP)
- Provider cost breakdown table (GCP vs OpenRouter)
- Tool call analysis (mcp_calls vs native_calls per condition)
- Effective cost per quality point: cost_usd / (quality_score * reliability)
- Protocol violations summary
- Cache and token metrics (exploratory)

Usage:
    python3 analyze.py --scores-file docs/benchmarks/v10/scores.json

Output: Markdown tables printed to stdout.
"""

import argparse
import json
import math
import statistics
import sys
from pathlib import Path
from typing import Dict, List, Tuple


def mannwhitneyu(
    group1: List[float], group2: List[float]
) -> Tuple[float, float, float]:
    """Compute Mann-Whitney U, z-score approximation, and p-value (scipy preferred)."""
    try:
        from scipy import stats

        result = stats.mannwhitneyu(group1, group2, alternative="two-sided")
        U = result.statistic
        p = result.pvalue
        n1, n2 = len(group1), len(group2)
        mu = n1 * n2 / 2
        sigma = math.sqrt(n1 * n2 * (n1 + n2 + 1) / 12)
        z = (U - mu) / sigma if sigma > 0 else 0.0
        return U, z, p
    except ImportError:
        n1, n2 = len(group1), len(group2)
        combined = sorted(
            [(v, g) for g, grp in enumerate([group1, group2]) for v in grp]
        )
        ranks: List[float] = []
        i = 0
        while i < len(combined):
            j = i
            while j < len(combined) and combined[j][0] == combined[i][0]:
                j += 1
            avg = (i + 1 + j) / 2
            ranks.extend([avg] * (j - i))
            i = j
        r1 = sum(ranks[k] for k in range(len(combined)) if combined[k][1] == 0)
        U = r1 - n1 * (n1 + 1) / 2
        mu = n1 * n2 / 2
        sigma = math.sqrt(n1 * n2 * (n1 + n2 + 1) / 12)
        z = (U - mu) / sigma if sigma > 0 else 0.0
        return U, z, float("nan")


def rank_biserial_r(U: float, n1: int, n2: int) -> float:
    return 1 - (2 * U) / (n1 * n2) if (n1 * n2) > 0 else 0.0


def split_conditions(
    per_run: Dict, field: str
) -> Tuple[
    List[float], List[float], List[float], List[float], List[float], List[float]
]:
    """Split scores by condition: A, A2, B, C, D, E."""
    a, a2, b, c, d, e = [], [], [], [], [], []
    for run_id, data in per_run.items():
        if data is None:
            continue
        val = data.get(field)
        if val is None and field == "total":
            # Prefer explicit quality_score field first
            val = data.get("quality_score")
        if val is None and field == "total":
            dims = [
                "structural_accuracy",
                "cross_module_tracing",
                "approach_quality",
                "tool_efficiency",
            ]
            vals = [data.get(d) for d in dims]
            if all(v is not None for v in vals):
                val = sum(vals)
        if val is not None:
            condition = "A2" if run_id.startswith("A2_") else run_id[0]
            val_float = float(val)
            if condition == "A2":
                a2.append(val_float)
            elif condition == "A":
                a.append(val_float)
            elif condition == "B":
                b.append(val_float)
            elif condition == "C":
                c.append(val_float)
            elif condition == "D":
                d.append(val_float)
            elif condition == "E":
                e.append(val_float)
    return a, a2, b, c, d, e


def split_metrics(
    per_run: Dict, metric: str
) -> Tuple[
    List[float], List[float], List[float], List[float], List[float], List[float]
]:
    """Split metrics by condition: A, A2, B, C, D, E."""
    a, a2, b, c, d, e = [], [], [], [], [], []
    for run_id, data in per_run.items():
        if data is None:
            continue
        metrics = data.get("metrics", {})
        val = metrics.get(metric) if metrics else None
        if val is not None:
            condition = "A2" if run_id.startswith("A2_") else run_id[0]
            val_float = float(val)
            if condition == "A2":
                a2.append(val_float)
            elif condition == "A":
                a.append(val_float)
            elif condition == "B":
                b.append(val_float)
            elif condition == "C":
                c.append(val_float)
            elif condition == "D":
                d.append(val_float)
            elif condition == "E":
                e.append(val_float)
    return a, a2, b, c, d, e


def print_quality_table(per_run: Dict):
    print("## Quality Analysis\n")
    print("Bonferroni alpha = 0.05 / 15 = 0.0033 (15 pairwise contrasts)\n")
    print("| Dimension | Med A | Med A2 | Med B | Med C | Med D | Med E |")
    print("|-----------|-------|--------|-------|-------|-------|-------|")

    dims = [
        "structural_accuracy",
        "cross_module_tracing",
        "approach_quality",
        "tool_efficiency",
    ]
    for dim in dims + ["total"]:
        a, a2, b, c, d, e = split_conditions(per_run, dim)
        if not (a and a2 and b and c and d and e):
            continue

        label = f"**{dim}**" if dim == "total" else dim
        med_a = f"{statistics.median(a):.1f}" if a else "n/a"
        med_a2 = f"{statistics.median(a2):.1f}" if a2 else "n/a"
        med_b = f"{statistics.median(b):.1f}" if b else "n/a"
        med_c = f"{statistics.median(c):.1f}" if c else "n/a"
        med_d = f"{statistics.median(d):.1f}" if d else "n/a"
        med_e = f"{statistics.median(e):.1f}" if e else "n/a"

        print(
            f"| {label} | {med_a} | {med_a2} | {med_b} | {med_c} | {med_d} | {med_e} |"
        )


def print_pairwise_tests(per_run: Dict):
    print("\n## Pairwise Mann-Whitney U Tests (15 contrasts)\n")
    print("| Contrast | U | z | p-value | r |")
    print("|----------|---|---|---------|---|")

    a, a2, b, c, d, e = split_conditions(per_run, "total")

    # All 15 pairwise combinations
    pairs = [
        ("A", "A2", a, a2),
        ("A", "B", a, b),
        ("A", "C", a, c),
        ("A", "D", a, d),
        ("A", "E", a, e),
        ("A2", "B", a2, b),
        ("A2", "C", a2, c),
        ("A2", "D", a2, d),
        ("A2", "E", a2, e),
        ("B", "C", b, c),
        ("B", "D", b, d),
        ("B", "E", b, e),
        ("C", "D", c, d),
        ("C", "E", c, e),
        ("D", "E", d, e),
    ]

    for label1, label2, group1, group2 in pairs:
        if not (group1 and group2):
            continue
        U, z, p = mannwhitneyu(group1, group2)
        r = rank_biserial_r(U, len(group1), len(group2))
        p_str = f"{p:.4f}" if not math.isnan(p) else "n/a"
        print(f"| {label1} vs {label2} | {U:.0f} | {z:.2f} | {p_str} | {r:.2f} |")


def print_factorial_table(per_run: Dict):
    print("\n## 2x2 Factorial Summary (Median Quality Score)\n")
    print("| Model | Native | MCP |")
    print("|-------|--------|-----|")

    a, a2, b, c, d, e = split_conditions(per_run, "total")

    # Rows: Sonnet (A, C), Haiku (A2, B), MiniMax (D), Mistral (E)
    # Cols: native (A, A2), MCP (B, C, D, E)

    sonnet_native = f"{statistics.median(a):.1f}" if a else "n/a"
    sonnet_mcp = f"{statistics.median(c):.1f}" if c else "n/a"
    print(f"| Sonnet | {sonnet_native} | {sonnet_mcp} |")

    haiku_native = f"{statistics.median(a2):.1f}" if a2 else "n/a"
    haiku_mcp = f"{statistics.median(b):.1f}" if b else "n/a"
    print(f"| Haiku | {haiku_native} | {haiku_mcp} |")

    minimax_native = "n/a"
    minimax_mcp = f"{statistics.median(d):.1f}" if d else "n/a"
    print(f"| MiniMax | {minimax_native} | {minimax_mcp} |")

    mistral_native = "n/a"
    mistral_mcp = f"{statistics.median(e):.1f}" if e else "n/a"
    print(f"| Mistral | {mistral_native} | {mistral_mcp} |")


def print_provider_cost_table(per_run: Dict):
    print("\n## Provider Cost Breakdown\n")
    print(
        "| Condition | Provider | Median cost_usd | Median quality_score | Effective cost/QP |"
    )
    print(
        "|-----------|----------|-----------------|----------------------|-------------------|"
    )

    a_costs, a2_costs, b_costs, c_costs, d_costs, e_costs = split_metrics(
        per_run, "cost_usd"
    )
    a_quality, a2_quality, b_quality, c_quality, d_quality, e_quality = (
        split_conditions(per_run, "total")
    )

    def format_cost(costs, quality, reliability=None):
        if not costs or not quality:
            return "n/a", "n/a", "n/a"
        med_cost = statistics.median(costs)
        med_qual = statistics.median(quality)
        rel = statistics.mean(reliability) if reliability else 1.0
        denom = med_qual * rel
        eff_cost = med_cost / denom if denom > 0 else float("inf")
        eff_cost_str = f"{eff_cost:.4f}" if eff_cost != float("inf") else "inf"
        return f"{med_cost:.4f}", f"{med_qual:.1f}", eff_cost_str

    a_rel, a2_rel, b_rel, c_rel, d_rel, e_rel = split_metrics(per_run, "valid_output")

    # GCP conditions: A, A2, B, C
    a_cost, a_qual, a_eff = format_cost(a_costs, a_quality, a_rel)
    print(f"| A | GCP | {a_cost} | {a_qual} | {a_eff} |")

    a2_cost, a2_qual, a2_eff = format_cost(a2_costs, a2_quality, a2_rel)
    print(f"| A2 | GCP | {a2_cost} | {a2_qual} | {a2_eff} |")

    b_cost, b_qual, b_eff = format_cost(b_costs, b_quality, b_rel)
    print(f"| B | GCP | {b_cost} | {b_qual} | {b_eff} |")

    c_cost, c_qual, c_eff = format_cost(c_costs, c_quality, c_rel)
    print(f"| C | GCP | {c_cost} | {c_qual} | {c_eff} |")

    # OpenRouter conditions: D, E
    d_cost, d_qual, d_eff = format_cost(d_costs, d_quality, d_rel)
    print(f"| D | OpenRouter | {d_cost} | {d_qual} | {d_eff} |")

    e_cost, e_qual, e_eff = format_cost(e_costs, e_quality, e_rel)
    print(f"| E | OpenRouter | {e_cost} | {e_qual} | {e_eff} |")


def print_tool_call_table(per_run: Dict):
    print("\n## Tool Call Analysis\n")
    print(
        "| Run | Condition | tool_calls_total | research_calls | mcp_calls | native_calls |"
    )
    print(
        "|-----|-----------|-----------------|----------------|-----------|--------------|"
    )
    for run_id in sorted(per_run.keys()):
        data = per_run[run_id]
        if data is None:
            continue
        metrics = data.get("metrics", {}) or {}
        cond = "A2" if run_id.startswith("A2_") else run_id[0]
        print(
            f"| {run_id} | {cond} | {metrics.get('tool_calls_total', '-')} | {metrics.get('research_calls', '-')} | {metrics.get('mcp_calls', '-')} | {metrics.get('native_calls', '-')} |"
        )


def print_protocol_violations(per_run: Dict):
    print("\n## Protocol Violations\n")
    (
        a_violations,
        a2_violations,
        b_violations,
        c_violations,
        d_violations,
        e_violations,
    ) = 0, 0, 0, 0, 0, 0
    for run_id, data in per_run.items():
        if data is None:
            continue
        metrics = data.get("metrics", {})
        violations = metrics.get("protocol_violations", 0) if metrics else 0
        condition = "A2" if run_id.startswith("A2_") else run_id[0]
        if condition == "A2":
            a2_violations += violations
        elif condition == "A":
            a_violations += violations
        elif condition == "B":
            b_violations += violations
        elif condition == "C":
            c_violations += violations
        elif condition == "D":
            d_violations += violations
        elif condition == "E":
            e_violations += violations

    print("| Condition | Total violations |")
    print("|-----------|------------------|")
    print(f"| A | {a_violations} |")
    print(f"| A2 | {a2_violations} |")
    print(f"| B | {b_violations} |")
    print(f"| C | {c_violations} |")
    print(f"| D | {d_violations} |")
    print(f"| E | {e_violations} |")


def print_cache_and_tokens(per_run: Dict):
    print(
        "\n## Cache Tokens and Total Tokens (exploratory — caching globally disabled)\n"
    )
    a_cache_w, a2_cache_w, b_cache_w, c_cache_w, d_cache_w, e_cache_w = split_metrics(
        per_run, "cache_write_tokens"
    )
    a_cache_r, a2_cache_r, b_cache_r, c_cache_r, d_cache_r, e_cache_r = split_metrics(
        per_run, "cache_read_tokens"
    )
    a_total, a2_total, b_total, c_total, d_total, e_total = split_metrics(
        per_run, "total_tokens"
    )

    print("| Condition | Med cache_write | Med cache_read | Med total_tokens |")
    print("|-----------|-----------------|----------------|------------------|")

    a_cw_str = f"{statistics.median(a_cache_w):.0f}" if a_cache_w else "n/a"
    a_cr_str = f"{statistics.median(a_cache_r):.0f}" if a_cache_r else "n/a"
    a_tot_str = f"{statistics.median(a_total):.0f}" if a_total else "n/a"
    print(f"| A | {a_cw_str} | {a_cr_str} | {a_tot_str} |")

    a2_cw_str = f"{statistics.median(a2_cache_w):.0f}" if a2_cache_w else "n/a"
    a2_cr_str = f"{statistics.median(a2_cache_r):.0f}" if a2_cache_r else "n/a"
    a2_tot_str = f"{statistics.median(a2_total):.0f}" if a2_total else "n/a"
    print(f"| A2 | {a2_cw_str} | {a2_cr_str} | {a2_tot_str} |")

    b_cw_str = f"{statistics.median(b_cache_w):.0f}" if b_cache_w else "n/a"
    b_cr_str = f"{statistics.median(b_cache_r):.0f}" if b_cache_r else "n/a"
    b_tot_str = f"{statistics.median(b_total):.0f}" if b_total else "n/a"
    print(f"| B | {b_cw_str} | {b_cr_str} | {b_tot_str} |")

    c_cw_str = f"{statistics.median(c_cache_w):.0f}" if c_cache_w else "n/a"
    c_cr_str = f"{statistics.median(c_cache_r):.0f}" if c_cache_r else "n/a"
    c_tot_str = f"{statistics.median(c_total):.0f}" if c_total else "n/a"
    print(f"| C | {c_cw_str} | {c_cr_str} | {c_tot_str} |")

    d_cw_str = f"{statistics.median(d_cache_w):.0f}" if d_cache_w else "n/a"
    d_cr_str = f"{statistics.median(d_cache_r):.0f}" if d_cache_r else "n/a"
    d_tot_str = f"{statistics.median(d_total):.0f}" if d_total else "n/a"
    print(f"| D | {d_cw_str} | {d_cr_str} | {d_tot_str} |")

    e_cw_str = f"{statistics.median(e_cache_w):.0f}" if e_cache_w else "n/a"
    e_cr_str = f"{statistics.median(e_cache_r):.0f}" if e_cache_r else "n/a"
    e_tot_str = f"{statistics.median(e_total):.0f}" if e_total else "n/a"
    print(f"| E | {e_cw_str} | {e_cr_str} | {e_tot_str} |")


def main():
    parser = argparse.ArgumentParser(
        description="Analyze v10 benchmark scores: 15-way Mann-Whitney U, factorial table, provider cost breakdown"
    )
    parser.add_argument(
        "--scores-file",
        type=Path,
        default=Path("scores.json"),
        help="Path to filled scores.json (default: scores.json in cwd)",
    )
    args = parser.parse_args()

    if not args.scores_file.exists():
        print(f"Error: {args.scores_file} not found", file=sys.stderr)
        sys.exit(1)

    with open(args.scores_file) as f:
        scores = json.load(f)

    per_run = scores.get("per_run_scores", {})

    print(f"# Analysis: {scores.get('benchmark', 'v10-benchmark')}\n")
    print(f"**Model A:** {scores.get('model_a', 'claude-sonnet-4-6')}")
    print(f"**Model A2:** {scores.get('model_a2', 'claude-haiku-4-5')}")
    print(f"**Model B:** {scores.get('model_b', 'claude-haiku-4-5')}")
    print(f"**Model C:** {scores.get('model_c', 'claude-sonnet-4-6')}")
    print(f"**Model D:** {scores.get('model_d', 'minimax/minimax-m2.5')}")
    print(f"**Model E:** {scores.get('model_e', 'mistralai/mistral-small-2603')}")
    print(f"**Target:** {scores.get('target_repo', 'TBD')}")
    print("**Runs:** 24 (4 per condition)\n")

    print_quality_table(per_run)
    print_pairwise_tests(per_run)
    print_factorial_table(per_run)
    print_provider_cost_table(per_run)
    print_tool_call_table(per_run)
    print_protocol_violations(per_run)
    print_cache_and_tokens(per_run)

    print("\n---")
    print(
        "*Mann-Whitney U: scipy used when available; otherwise normal approximation (p shown as n/a).*"
    )
    print("*rank-biserial r: |r| >= 0.3 small effect, >= 0.5 medium, >= 0.7 large.*")
    print("*Bonferroni alpha = 0.0033 per test (0.05 / 15 pairwise contrasts).*")
    print("*All analyses reported as exploratory (N=4 per condition).*")


if __name__ == "__main__":
    main()
