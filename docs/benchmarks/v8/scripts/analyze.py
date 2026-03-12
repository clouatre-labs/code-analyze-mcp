#!/usr/bin/env python3
"""
analyze.py: Statistical analysis for v8 benchmark scores.

Reads a filled scores.json and computes:
- Per-condition medians and ranges for quality dimensions
- Mann-Whitney U test on total scores and each dimension
- Rank-biserial r (effect size)
- Tool call analysis (mcp_calls vs native_calls in B)
- Effective cost per quality point: cost_usd / (quality_score * reliability)

Usage:
    python3 analyze.py --scores-file docs/benchmarks/v8/scores.json

Output: Markdown tables printed to stdout.
"""

import argparse
import json
import math
import statistics
import sys
from pathlib import Path
from typing import Dict, List, Tuple


def mannwhitneyu(group1: List[float], group2: List[float]) -> Tuple[float, float, float]:
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
        combined = sorted([(v, g) for g, grp in enumerate([group1, group2]) for v in grp])
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


def split_conditions(per_run: Dict, field: str) -> Tuple[List[float], List[float]]:
    a, b = [], []
    for run_id, data in per_run.items():
        if data is None:
            continue
        val = data.get(field)
        if val is None and field == "total":
            dims = ["structural_accuracy", "cross_module_tracing", "approach_quality", "tool_efficiency"]
            vals = [data.get(d) for d in dims]
            if all(v is not None for v in vals):
                val = sum(vals)
        if val is not None:
            (a if run_id.startswith("A") else b).append(float(val))
    return a, b


def split_efficiency(per_run: Dict, metric: str) -> Tuple[List[float], List[float]]:
    a, b = [], []
    for run_id, data in per_run.items():
        if data is None:
            continue
        eff = data.get("efficiency", {})
        val = eff.get(metric) if eff else None
        if val is not None:
            (a if run_id.startswith("A") else b).append(float(val))
    return a, b


def print_quality_table(per_run: Dict):
    print("## Quality Analysis\n")
    print("| Dimension | Med A | Med B | U | z | p | r |")
    print("|-----------|-------|-------|---|---|---|---|")

    dims = ["structural_accuracy", "cross_module_tracing", "approach_quality", "tool_efficiency"]
    for dim in dims + ["total"]:
        a, b = split_conditions(per_run, dim)
        if not a or not b:
            continue
        U, z, p = mannwhitneyu(a, b)
        r = rank_biserial_r(U, len(a), len(b))
        label = f"**{dim}**" if dim == "total" else dim
        p_str = f"{p:.3f}" if not math.isnan(p) else "n/a"
        print(f"| {label} | {statistics.median(a):.1f} | {statistics.median(b):.1f} | {U:.1f} | {z:.2f} | {p_str} | {r:.2f} |")


def print_tool_call_table(per_run: Dict):
    print("\n## Tool Call Analysis\n")
    print("| Run | Condition | tool_calls_total | mcp_calls | native_calls |")
    print("|-----|-----------|-----------------|-----------|--------------|")
    for run_id in sorted(per_run.keys()):
        data = per_run[run_id]
        if data is None:
            continue
        eff = data.get("efficiency", {}) or {}
        print(f"| {run_id} | {'A' if run_id.startswith('A') else 'B'} | {eff.get('tool_calls_total', '-')} | {eff.get('mcp_calls', '-')} | {eff.get('native_calls', '-')} |")


def print_cost_table(per_run: Dict):
    print("\n## Cost and Effective Cost per Quality Point\n")
    print("effective_cost_per_qp = cost_usd / (quality_score * reliability)\n")

    a_valid, b_valid = 0, 0
    a_total, b_total = 0, 0
    for run_id, data in per_run.items():
        if data is None:
            continue
        if run_id.startswith("A"):
            a_total += 1
            if data.get("efficiency", {}).get("valid_output"):
                a_valid += 1
        else:
            b_total += 1
            if data.get("efficiency", {}).get("valid_output"):
                b_valid += 1

    rel_a = a_valid / a_total if a_total > 0 else 0.0
    rel_b = b_valid / b_total if b_total > 0 else 0.0

    a_costs, b_costs = split_efficiency(per_run, "cost_usd")
    a_quality, b_quality = split_conditions(per_run, "total")

    def eff_cost(costs, quality, reliability):
        if not costs or not quality or reliability == 0:
            return "n/a"
        median_cost = statistics.median(costs)
        median_quality = statistics.median(quality)
        return f"{median_cost / (median_quality * reliability):.4f}"

    print(f"| Condition | Reliability | Median cost_usd | Median quality | Eff. cost/QP |")
    print(f"|-----------|-------------|-----------------|----------------|--------------|")
    a_cost_str = f"{statistics.median(a_costs):.4f}" if a_costs else "n/a"
    b_cost_str = f"{statistics.median(b_costs):.4f}" if b_costs else "n/a"
    a_qual_str = f"{statistics.median(a_quality):.1f}" if a_quality else "n/a"
    b_qual_str = f"{statistics.median(b_quality):.1f}" if b_quality else "n/a"
    print(f"| A | {rel_a:.2f} | {a_cost_str} | {a_qual_str} | {eff_cost(a_costs, a_quality, rel_a)} |")
    print(f"| B | {rel_b:.2f} | {b_cost_str} | {b_qual_str} | {eff_cost(b_costs, b_quality, rel_b)} |")


def main():
    parser = argparse.ArgumentParser(
        description="Analyze v8 benchmark scores: Mann-Whitney U, tool call analysis, effective cost/QP"
    )
    parser.add_argument(
        "--scores-file",
        type=Path,
        default=Path("scores.json"),
        help="Path to filled scores.json (default: scores.json in cwd)"
    )
    args = parser.parse_args()

    if not args.scores_file.exists():
        print(f"Error: {args.scores_file} not found", file=sys.stderr)
        sys.exit(1)

    with open(args.scores_file) as f:
        scores = json.load(f)

    per_run = scores.get("per_run_scores", {})

    print(f"# Analysis: {scores.get('experiment', 'v8-benchmark')}\n")
    print(f"**Model:** {scores.get('model')}")
    print(f"**Target:** {scores.get('target_repo')}")
    print(f"**Runs:** 10 (5 per condition)\n")

    print_quality_table(per_run)
    print_tool_call_table(per_run)
    print_cost_table(per_run)

    print("\n---")
    print("*Mann-Whitney U: scipy used when available; otherwise normal approximation (p shown as n/a).*")
    print("*rank-biserial r: |r| >= 0.3 small, >= 0.5 medium, >= 0.7 large.*")


if __name__ == "__main__":
    main()
