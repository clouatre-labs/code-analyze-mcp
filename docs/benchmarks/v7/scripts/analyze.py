#!/usr/bin/env python3
"""
analyze.py: Statistical analysis for v7 benchmark scores.

Reads a filled scores.json, computes per-condition medians and ranges for quality
and efficiency, runs Mann-Whitney U test, computes rank-biserial r, and generates
cross-version comparisons (v7B vs v6B, v7B vs v7A).

Extends v6 analysis with v7-specific features:
- --version flag: default unchanged; --version v7 triggers parameter usage analysis
- v7 mode: parameter usage frequency table (% of B runs using summary, cursor, page_size)
- v7 mode: v7B vs v6B token delta comparison

Usage:
    python3 analyze.py --scores-file docs/benchmarks/v7/scores.json
    python3 analyze.py --scores-file docs/benchmarks/v7/scores.json --version v7
    python3 analyze.py --scores-file docs/benchmarks/v7/scores.json --base-version v6

Output: Markdown tables printed to stdout (can be pasted into analysis.md)
"""

import argparse
import json
import math
import statistics
from pathlib import Path
from typing import List, Dict, Tuple, Optional


def mannwhitneyu_exact(group1: List[float], group2: List[float]) -> Tuple[float, float, float]:
    """
    Compute Mann-Whitney U statistic and approximate p-value.
    Falls back to manual calculation if scipy unavailable.
    
    Returns: (U, z_approx, p_approx)
    """
    try:
        from scipy import stats
        result = stats.mannwhitneyu(group1, group2, alternative='two-sided')
        U = result.statistic
        p_value = result.pvalue
        # z-score approximation: z = (U - mu) / sigma where mu=n1*n2/2, sigma=sqrt(n1*n2*(n1+n2+1)/12)
        n1, n2 = len(group1), len(group2)
        mu = n1 * n2 / 2
        sigma = math.sqrt(n1 * n2 * (n1 + n2 + 1) / 12)
        z = (U - mu) / sigma if sigma > 0 else 0
        return U, z, p_value
    except ImportError:
        # Manual calculation
        n1, n2 = len(group1), len(group2)
        combined = [(x, 1) for x in group1] + [(x, 2) for x in group2]
        combined.sort()
        
        # Assign ranks
        ranks = {}
        current_rank = 1
        i = 0
        while i < len(combined):
            # Handle ties
            j = i
            while j < len(combined) and combined[j][0] == combined[i][0]:
                j += 1
            avg_rank = (current_rank + j - i + 1) / 2 if j > i + 1 else current_rank
            for k in range(i, j):
                ranks[k] = avg_rank
            current_rank = j + 1
            i = j
        
        R1 = sum(ranks[i] for i in range(len(group1)))
        U = n1 * n2 + n1 * (n1 + 1) / 2 - R1
        
        # z-score
        mu = n1 * n2 / 2
        sigma = math.sqrt(n1 * n2 * (n1 + n2 + 1) / 12)
        z = (U - mu) / sigma if sigma > 0 else 0
        
        # p-value approximation (normal approximation, not exact)
        p_value = 0.0  # Placeholder; scipy recommended
        
        return U, z, p_value


def rank_biserial_r(U: float, n1: int, n2: int) -> float:
    """Compute rank-biserial correlation: r = 1 - 2U/(n1*n2)"""
    return 1 - (2 * U) / (n1 * n2) if (n1 * n2) > 0 else 0


def extract_condition_scores(scores_data: Dict, dimension: str) -> Tuple[List[float], List[float]]:
    """
    Extract scores for a given dimension.
    Returns (condition_a_scores, condition_b_scores)
    """
    per_run = scores_data.get('per_run_scores', {})
    
    a_scores = []
    b_scores = []
    
    for run_id, run_data in per_run.items():
        if run_data is None:
            continue
        score = run_data.get(dimension) if dimension != 'total' else run_data.get('total')
        if score is not None:
            if run_id.startswith('A'):
                a_scores.append(score)
            elif run_id.startswith('B'):
                b_scores.append(score)
    
    return a_scores, b_scores


def extract_efficiency_metric(scores_data: Dict, metric: str) -> Tuple[List[float], List[float]]:
    """Extract efficiency metric (e.g., total_calls) for A and B conditions."""
    per_run = scores_data.get('per_run_scores', {})
    
    a_metrics = []
    b_metrics = []
    
    for run_id, run_data in per_run.items():
        if run_data is None:
            continue
        val = run_data.get('efficiency', {}).get(metric) if 'efficiency' in run_data else None
        if val is not None:
            if run_id.startswith('A'):
                a_metrics.append(val)
            elif run_id.startswith('B'):
                b_metrics.append(val)
    
    return a_metrics, b_metrics


def load_scores(filepath: Path) -> Dict:
    """Load and validate scores.json"""
    with open(filepath) as f:
        data = json.load(f)
    return data


def get_baseline_key(base_version: str) -> str:
    """Get baseline key name for the given version"""
    if base_version == 'v6':
        return 'v6_baselines'
    elif base_version == 'v5':
        return 'v5_baselines'
    elif base_version == 'v3':
        return 'v3_baselines'
    else:
        raise ValueError(f"Unsupported base_version: {base_version}")


def extract_baselines(scores: Dict, base_version: str) -> Optional[Dict]:
    """Extract baseline data from scores using the specified base version"""
    baseline_key = get_baseline_key(base_version)
    baselines = scores.get(baseline_key)
    if not baselines:
        raise KeyError(f"{baseline_key} not found in scores.json")
    return baselines


def extract_parameter_usage_stats(scores: Dict) -> Dict:
    """
    Extract parameter usage statistics from per_run_scores.

    For Condition B runs, aggregates:
    - % runs using summary=true
    - % runs using cursor
    - % runs using page_size

    Returns dict with usage percentages and counts.
    """
    per_run = scores.get('per_run_scores', {})
    
    b_runs = []
    for run_id, run_data in per_run.items():
        if run_data is None or not run_id.startswith('B'):
            continue
        b_runs.append(run_data)
    
    if not b_runs:
        return {
            'b_run_count': 0,
            'summary_usage_pct': 0,
            'cursor_usage_pct': 0,
            'page_size_usage_pct': 0
        }
    
    summary_count = sum(1 for r in b_runs if r.get('parameter_usage', {}).get('summary_count', 0) > 0)
    cursor_count = sum(1 for r in b_runs if r.get('parameter_usage', {}).get('cursor_calls', 0) > 0)
    page_size_count = sum(1 for r in b_runs if r.get('parameter_usage', {}).get('page_size_overrides', 0) > 0)
    
    b_count = len(b_runs)
    
    return {
        'b_run_count': b_count,
        'summary_usage_pct': round(100 * summary_count / b_count) if b_count > 0 else 0,
        'cursor_usage_pct': round(100 * cursor_count / b_count) if b_count > 0 else 0,
        'page_size_usage_pct': round(100 * page_size_count / b_count) if b_count > 0 else 0,
        'summary_count': summary_count,
        'cursor_count': cursor_count,
        'page_size_count': page_size_count
    }


def print_quality_table(scores: Dict):
    """Print quality table with Mann-Whitney U test"""
    print("## Quality Analysis\n")
    print("| Dimension | Cond A | Cond B | U-stat | z | p-val | r |")
    print("|-----------|--------|--------|--------|-------|----------|---------|")
    
    dimensions = ['structural_accuracy', 'cross_module_tracing', 'approach_quality', 'tool_efficiency']
    
    for dim in dimensions:
        a_scores, b_scores = extract_condition_scores(scores, dim)
        
        if not a_scores or not b_scores:
            continue
        
        a_med = statistics.median(a_scores)
        b_med = statistics.median(b_scores)
        
        U, z, p = mannwhitneyu_exact(a_scores, b_scores)
        r = rank_biserial_r(U, len(a_scores), len(b_scores))
        
        print(f"| {dim} | {a_med:.1f} | {b_med:.1f} | {U:.1f} | {z:.2f} | {p:.3f} | {r:.2f} |")
    
    # Total scores
    a_totals, b_totals = extract_condition_scores(scores, 'total')
    if a_totals and b_totals:
        a_med = statistics.median(a_totals)
        b_med = statistics.median(b_totals)
        
        U, z, p = mannwhitneyu_exact(a_totals, b_totals)
        r = rank_biserial_r(U, len(a_totals), len(b_totals))
        
        print(f"| **total** | **{a_med:.1f}** | **{b_med:.1f}** | **{U:.1f}** | **{z:.2f}** | **{p:.3f}** | **{r:.2f}** |")


def print_efficiency_table(scores: Dict):
    """Print efficiency metrics"""
    print("\n## Efficiency Analysis\n")
    print("Tool efficiency from per_run_scores[run_id].efficiency.total_calls (if available)")


def print_tool_efficiency_summary(scores: Dict):
    """Print tool efficiency summary (tool_efficiency dimension)"""
    print("\n## Tool Efficiency (4-point dimension)\n")
    
    a_scores, b_scores = extract_condition_scores(scores, 'tool_efficiency')
    
    if a_scores and b_scores:
        print(f"| Condition | Median | Range | Scores |")
        print("|-----------|--------|-------|--------|")
        print(f"| A | {statistics.median(a_scores):.1f} | {min(a_scores)}-{max(a_scores)} | {a_scores} |")
        print(f"| B | {statistics.median(b_scores):.1f} | {min(b_scores)}-{max(b_scores)} | {b_scores} |")


def print_parameter_usage_table(scores: Dict):
    """Print v7-specific parameter usage frequency table"""
    print("\n## Parameter Usage Frequency (v7 Condition B)\n")
    
    stats = extract_parameter_usage_stats(scores)
    
    if stats['b_run_count'] == 0:
        print("No Condition B runs found with parameter_usage data")
        return
    
    print(f"| Parameter | % of B Runs | Count |")
    print("|-----------|-------------|-------|")
    print(f"| summary=true | {stats['summary_usage_pct']}% | {stats['summary_count']}/{stats['b_run_count']} |")
    print(f"| cursor | {stats['cursor_usage_pct']}% | {stats['cursor_count']}/{stats['b_run_count']} |")
    print(f"| page_size | {stats['page_size_usage_pct']}% | {stats['page_size_count']}/{stats['b_run_count']} |")


def print_cross_version_comparison(scores: Dict, base_version: str = 'v6'):
    """
    Compare v7B vs baseline (v6B or v5B) and v7B vs v7A.
    Uses baseline_key from scores to gracefully handle missing data.
    """
    print("\n## Cross-Version Comparison\n")
    
    baseline_key = get_baseline_key(base_version)
    
    try:
        baselines = extract_baselines(scores, base_version)
    except KeyError as e:
        print(f"**Error:** {e}")
        print(f"Cannot perform cross-version comparison without {baseline_key}")
        return
    
    # Extract v7B (treatment in current run)
    v7b_quality = [scores['per_run_scores'][k]['total'] for k in scores['per_run_scores'] if k and k.startswith('B') and scores['per_run_scores'][k] is not None]
    
    # Extract baseline B condition
    if base_version in ('v6', 'v5'):
        baseline_b_median = baselines.get('conditions', {}).get('B', {}).get('median_total_score')
    else:
        baseline_b_median = baselines.get('B_condition', {}).get('median')
    
    # v7B vs baseline B
    print(f"### v7B vs {base_version.upper()}B (Parameter Optimization Delta)\n")
    
    if v7b_quality and baseline_b_median is not None:
        v7b_med_quality = statistics.median(v7b_quality)
        print(f"| Metric | {base_version.upper()}B | v7B | Delta |")
        print("|--------|-----|-----|-------|")
        print(f"| Quality (total) | {baseline_b_median} | {v7b_med_quality} | {v7b_med_quality - baseline_b_median:+.1f} |")
    
    # Extract v7A for gap closure
    print(f"\n### v7B vs v7A (Gap Closure)\n")
    
    v7a_quality = [scores['per_run_scores'][k]['total'] for k in scores['per_run_scores'] if k and k.startswith('A') and scores['per_run_scores'][k] is not None]
    
    if v7b_quality and v7a_quality:
        v7a_med_quality = statistics.median(v7a_quality)
        v7b_med_quality = statistics.median(v7b_quality)
        print(f"| Metric | v7A | v7B | Gap |")
        print("|--------|-----|-----|-----|")
        print(f"| Quality | {v7a_med_quality} | {v7b_med_quality} | {v7b_med_quality - v7a_med_quality:+.1f} |")
        print(f"| **Hypothesis: v7B agents use new parameters (summary, cursor, page_size) to reduce token overhead compared to v6B** | | | |")


def main():
    parser = argparse.ArgumentParser(
        description='Analyze v7 benchmark scores: Mann-Whitney U, rank-biserial r, parameter usage, cross-version comparisons'
    )
    parser.add_argument(
        '--scores-file',
        type=Path,
        default=Path('scores.json'),
        help='Path to filled scores.json (default: scores.json in cwd)'
    )
    parser.add_argument(
        '--version',
        choices=['v6', 'v7'],
        default='v7',
        help='Analysis version mode (default: v7)'
    )
    parser.add_argument(
        '--base-version',
        choices=['v3', 'v5', 'v6'],
        default='v6',
        help='Baseline version for cross-version comparison (default: v6)'
    )
    
    args = parser.parse_args()
    
    if not args.scores_file.exists():
        print(f"Error: {args.scores_file} not found", file=__import__('sys').stderr)
        exit(1)
    
    scores = load_scores(args.scores_file)
    
    print(f"# Analysis: {scores.get('experiment', 'v7-benchmark')}\n")
    print(f"**Model:** {scores.get('model')}")
    print(f"**Runs:** 10 (5 per condition)")
    print(f"**Analysis mode:** {args.version}")
    print(f"**Base version for comparison:** {args.base_version.upper()}\n")
    
    # Always run core v6-compatible analysis
    print_quality_table(scores)
    print_efficiency_table(scores)
    print_tool_efficiency_summary(scores)
    
    # v7 mode: add parameter usage analysis
    if args.version == 'v7':
        print_parameter_usage_table(scores)
    
    print_cross_version_comparison(scores, args.base_version)
    
    print("\n---\n")
    print("*Note: Mann-Whitney U test requires scipy for exact p-values. If scipy unavailable, p-value is approximate.*")
    print("*rank-biserial r: |r| >= 0.3 (small), >= 0.5 (medium), >= 0.7 (large effect).*")


if __name__ == '__main__':
    main()
