#!/usr/bin/env python3
"""
analyze.py: Statistical analysis for v9 benchmark scores (3 conditions).

Reads a filled scores.json and computes:
- Per-condition medians and ranges for quality dimensions
- 3 pairwise Mann-Whitney U tests (A vs B, A vs C, B vs C) with Bonferroni correction
- Rank-biserial r (effect size)
- Tool call analysis (mcp_calls vs native_calls per condition)
- Effective cost per quality point: cost_usd / (quality_score * reliability)
- Protocol violations summary
- Cache and token metrics (exploratory)

Usage:
    python3 analyze.py --scores-file docs/benchmarks/v9/scores.json

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


def split_conditions(per_run: Dict, field: str) -> Tuple[List[float], List[float], List[float]]:
    a, b, c = [], [], []
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
            condition = run_id[0]
            if condition == "A":
                a.append(float(val))
            elif condition == "B":
                b.append(float(val))
            elif condition == "C":
                c.append(float(val))
    return a, b, c


def split_metrics(per_run: Dict, metric: str) -> Tuple[List[float], List[float], List[float]]:
    a, b, c = [], [], []
    for run_id, data in per_run.items():
        if data is None:
            continue
        metrics = data.get("metrics", {})
        val = metrics.get(metric) if metrics else None
        if val is not None:
            condition = run_id[0]
            if condition == "A":
                a.append(float(val))
            elif condition == "B":
                b.append(float(val))
            elif condition == "C":
                c.append(float(val))
    return a, b, c


def print_quality_table(per_run: Dict):
    print("## Quality Analysis\n")
    print("Bonferroni alpha = 0.05 / 3 = 0.017 (3 pairwise contrasts)\n")
    print("| Dimension | Med A | Med B | Med C | A vs B (U, z, p, r) | A vs C (U, z, p, r) | B vs C (U, z, p, r) |")
    print("|-----------|-------|-------|-------|----------------------|----------------------|----------------------|")

    dims = ["structural_accuracy", "cross_module_tracing", "approach_quality", "tool_efficiency"]
    for dim in dims + ["total"]:
        a, b, c = split_conditions(per_run, dim)
        if not (a and b and c):
            continue

        U_ab, z_ab, p_ab = mannwhitneyu(a, b)
        r_ab = rank_biserial_r(U_ab, len(a), len(b))

        U_ac, z_ac, p_ac = mannwhitneyu(a, c)
        r_ac = rank_biserial_r(U_ac, len(a), len(c))

        U_bc, z_bc, p_bc = mannwhitneyu(b, c)
        r_bc = rank_biserial_r(U_bc, len(b), len(c))

        label = f"**{dim}**" if dim == "total" else dim
        p_ab_str = f"{p_ab:.3f}" if not math.isnan(p_ab) else "n/a"
        p_ac_str = f"{p_ac:.3f}" if not math.isnan(p_ac) else "n/a"
        p_bc_str = f"{p_bc:.3f}" if not math.isnan(p_bc) else "n/a"

        med_a = f"{statistics.median(a):.1f}" if a else "n/a"
        med_b = f"{statistics.median(b):.1f}" if b else "n/a"
        med_c = f"{statistics.median(c):.1f}" if c else "n/a"

        print(f"| {label} | {med_a} | {med_b} | {med_c} | {U_ab:.0f}, {z_ab:.2f}, {p_ab_str}, {r_ab:.2f} | {U_ac:.0f}, {z_ac:.2f}, {p_ac_str}, {r_ac:.2f} | {U_bc:.0f}, {z_bc:.2f}, {p_bc_str}, {r_bc:.2f} |")


def print_tool_call_table(per_run: Dict):
    print("\n## Tool Call Analysis\n")
    print("| Run | Condition | tool_calls_total | research_calls | mcp_calls | native_calls |")
    print("|-----|-----------|-----------------|----------------|-----------|--------------|")
    for run_id in sorted(per_run.keys()):
        data = per_run[run_id]
        if data is None:
            continue
        metrics = data.get("metrics", {}) or {}
        cond = run_id[0]
        print(f"| {run_id} | {cond} | {metrics.get('tool_calls_total', '-')} | {metrics.get('research_calls', '-')} | {metrics.get('mcp_calls', '-')} | {metrics.get('native_calls', '-')} |")


def print_cost_table(per_run: Dict):
    print("\n## Cost and Effective Cost per Quality Point\n")
    print("effective_cost_per_qp = median_cost_usd / (median_quality_score * reliability)\n")
    print("reliability = fraction of valid_output runs per condition\n")

    a_valid, b_valid, c_valid = 0, 0, 0
    a_total, b_total, c_total = 0, 0, 0
    for run_id, data in per_run.items():
        if data is None:
            continue
        metrics = data.get("metrics", {})
        valid = metrics.get("valid_output") if metrics else False
        condition = run_id[0]
        if condition == "A":
            a_total += 1
            if valid:
                a_valid += 1
        elif condition == "B":
            b_total += 1
            if valid:
                b_valid += 1
        elif condition == "C":
            c_total += 1
            if valid:
                c_valid += 1

    rel_a = a_valid / a_total if a_total > 0 else 0.0
    rel_b = b_valid / b_total if b_total > 0 else 0.0
    rel_c = c_valid / c_total if c_total > 0 else 0.0

    a_costs, b_costs, c_costs = split_metrics(per_run, "cost_usd")
    a_quality, b_quality, c_quality = split_conditions(per_run, "total")

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
    c_cost_str = f"{statistics.median(c_costs):.4f}" if c_costs else "n/a"
    a_qual_str = f"{statistics.median(a_quality):.1f}" if a_quality else "n/a"
    b_qual_str = f"{statistics.median(b_quality):.1f}" if b_quality else "n/a"
    c_qual_str = f"{statistics.median(c_quality):.1f}" if c_quality else "n/a"

    print(f"| A (Sonnet, native) | {rel_a:.2f} | {a_cost_str} | {a_qual_str} | {eff_cost(a_costs, a_quality, rel_a)} |")
    print(f"| B (Haiku, MCP) | {rel_b:.2f} | {b_cost_str} | {b_qual_str} | {eff_cost(b_costs, b_quality, rel_b)} |")
    print(f"| C (Sonnet, MCP) | {rel_c:.2f} | {c_cost_str} | {c_qual_str} | {eff_cost(c_costs, c_quality, rel_c)} |")


def print_protocol_violations(per_run: Dict):
    print("\n## Protocol Violations\n")
    a_violations, b_violations, c_violations = 0, 0, 0
    for run_id, data in per_run.items():
        if data is None:
            continue
        metrics = data.get("metrics", {})
        violations = metrics.get("protocol_violations", 0) if metrics else 0
        condition = run_id[0]
        if condition == "A":
            a_violations += violations
        elif condition == "B":
            b_violations += violations
        elif condition == "C":
            c_violations += violations

    print(f"| Condition | Total violations |")
    print(f"|-----------|------------------|")
    print(f"| A | {a_violations} |")
    print(f"| B | {b_violations} |")
    print(f"| C | {c_violations} |")


def print_cache_and_tokens(per_run: Dict):
    print("\n## Cache Tokens and Total Tokens (exploratory — caching globally disabled)\n")
    a_cache_w, b_cache_w, c_cache_w = split_metrics(per_run, "cache_write_tokens")
    a_cache_r, b_cache_r, c_cache_r = split_metrics(per_run, "cache_read_tokens")
    a_total, b_total, c_total = split_metrics(per_run, "total_tokens")

    print(f"| Condition | Med cache_write | Med cache_read | Med total_tokens |")
    print(f"|-----------|-----------------|----------------|------------------|")

    a_cw_str = f"{statistics.median(a_cache_w):.0f}" if a_cache_w else "n/a"
    b_cw_str = f"{statistics.median(b_cache_w):.0f}" if b_cache_w else "n/a"
    c_cw_str = f"{statistics.median(c_cache_w):.0f}" if c_cache_w else "n/a"

    a_cr_str = f"{statistics.median(a_cache_r):.0f}" if a_cache_r else "n/a"
    b_cr_str = f"{statistics.median(b_cache_r):.0f}" if b_cache_r else "n/a"
    c_cr_str = f"{statistics.median(c_cache_r):.0f}" if c_cache_r else "n/a"

    a_tot_str = f"{statistics.median(a_total):.0f}" if a_total else "n/a"
    b_tot_str = f"{statistics.median(b_total):.0f}" if b_total else "n/a"
    c_tot_str = f"{statistics.median(c_total):.0f}" if c_total else "n/a"

    print(f"| A | {a_cw_str} | {a_cr_str} | {a_tot_str} |")
    print(f"| B | {b_cw_str} | {b_cr_str} | {b_tot_str} |")
    print(f"| C | {c_cw_str} | {c_cr_str} | {c_tot_str} |")


def main():
    parser = argparse.ArgumentParser(
        description="Analyze v9 benchmark scores: 3-way Mann-Whitney U, tool call analysis, effective cost/QP"
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

    print(f"# Analysis: {scores.get('benchmark', 'v9-benchmark')}\n")
    print(f"**Model A:** {scores.get('model_a', 'claude-sonnet-4-6')}")
    print(f"**Model B:** {scores.get('model_b', 'claude-haiku-4-5')}")
    print(f"**Model C:** {scores.get('model_c', 'claude-sonnet-4-6')}")
    print(f"**Target:** {scores.get('target_repo', 'TBD')}")
    print(f"**Runs:** 15 (5 per condition)\n")

    print_quality_table(per_run)
    print_tool_call_table(per_run)
    print_cost_table(per_run)
    print_protocol_violations(per_run)
    print_cache_and_tokens(per_run)

    print("\n---")
    print("*Mann-Whitney U: scipy used when available; otherwise normal approximation (p shown as n/a).*")
    print("*rank-biserial r: |r| >= 0.3 small effect, >= 0.5 medium, >= 0.7 large.*")
    print("*Bonferroni alpha = 0.017 per test (0.05 / 3 pairwise contrasts).*")
    print("*All analyses reported as exploratory (N=5 per condition).*")


if __name__ == "__main__":
    main()
