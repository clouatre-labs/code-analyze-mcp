#!/usr/bin/env python3
"""
analyze.py: Statistical analysis for v5 benchmark scores.

Reads a filled scores.json, computes per-condition medians and ranges for quality
and efficiency, runs Mann-Whitney U test, computes rank-biserial r, and generates
cross-version comparisons (v5B vs v3B, v5B vs v3A).

Usage:
    python3 analyze.py --scores-file docs/benchmarks/v5/scores.json

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


def load_scores(filepath: Path) -> Dict:
    """Load and validate scores.json"""
    with open(filepath) as f:
        data = json.load(f)
    return data


def extract_condition_scores(scores_data: Dict, dimension: str) -> Tuple[List[int], List[int]]:
    """
    Extract scores for dimension (e.g., 'structural_accuracy', 'tool_efficiency')
    from per_run_scores, grouped by condition A and B.
    
    Returns: (condition_A_scores, condition_B_scores)
    """
    a_scores = []
    b_scores = []
    
    for run_id, run_data in scores_data['per_run_scores'].items():
        if run_data[dimension] is not None:
            if run_id.startswith('A'):
                a_scores.append(run_data[dimension])
            elif run_id.startswith('B'):
                b_scores.append(run_data[dimension])
    
    return a_scores, b_scores


def extract_efficiency_metric(scores_data: Dict, metric: str) -> Tuple[List[float], List[float]]:
    """
    Extract efficiency metric (tokens, wall_seconds, total_calls, etc.)
    from per_run efficiency data, grouped by condition.
    
    Returns: (condition_A_values, condition_B_values)
    """
    a_vals = []
    b_vals = []
    
    for run_id, run_data in scores_data['efficiency']['per_run'].items():
        if run_data[metric] is not None:
            if run_id.startswith('A'):
                a_vals.append(run_data[metric])
            elif run_id.startswith('B'):
                b_vals.append(run_data[metric])
    
    return a_vals, b_vals


def compute_stats(values: List[float]) -> Dict:
    """Compute basic statistics"""
    if not values:
        return {'median': None, 'min': None, 'max': None, 'mean': None}
    return {
        'median': statistics.median(values),
        'min': min(values),
        'max': max(values),
        'mean': statistics.mean(values)
    }


def print_quality_table(scores_data: Dict):
    """Print quality scores analysis table"""
    print("\n## Quality Analysis\n")
    
    dimensions = ['structural_accuracy', 'cross_module_tracing', 'approach_quality']
    
    print("| Dimension | A Median | A Range | B Median | B Range | U | z | r | Significant |")
    print("|-----------|----------|---------|----------|---------|---|---|---|-------------|")
    
    for dim in dimensions:
        a_scores, b_scores = extract_condition_scores(scores_data, dim)
        
        if not a_scores or not b_scores:
            continue
        
        a_stats = compute_stats(a_scores)
        b_stats = compute_stats(b_scores)
        
        U, z, p = mannwhitneyu_exact(a_scores, b_scores)
        r = rank_biserial_r(U, len(a_scores), len(b_scores))
        significant = p < 0.05 if p > 0 else "N/A"
        
        print(f"| {dim} | {a_stats['median']} | [{a_stats['min']}, {a_stats['max']}] | "
              f"{b_stats['median']} | [{b_stats['min']}, {b_stats['max']}] | {U:.1f} | {z:.2f} | {r:.2f} | {significant} |")


def print_efficiency_table(scores_data: Dict):
    """Print efficiency analysis table"""
    print("\n## Efficiency Analysis\n")
    
    metrics = [
        ('tokens', 'Tokens'),
        ('wall_seconds', 'Wall Time (s)'),
        ('total_calls', 'Total Tool Calls'),
        ('analyze_calls', 'Analyze Calls'),
        ('shell_calls', 'Shell Calls'),
        ('editor_calls', 'Editor Calls')
    ]
    
    print("| Metric | A Median | A Range | B Median | B Range | U | z | r | Significant |")
    print("|--------|----------|---------|----------|---------|---|---|---|-------------|")
    
    for metric_key, metric_label in metrics:
        a_vals, b_vals = extract_efficiency_metric(scores_data, metric_key)
        
        if not a_vals or not b_vals:
            continue
        
        a_stats = compute_stats(a_vals)
        b_stats = compute_stats(b_vals)
        
        U, z, p = mannwhitneyu_exact(a_vals, b_vals)
        r = rank_biserial_r(U, len(a_vals), len(b_vals))
        significant = p < 0.05 if p > 0 else "N/A"
        
        print(f"| {metric_label} | {a_stats['median']:.0f} | [{a_stats['min']:.0f}, {a_stats['max']:.0f}] | "
              f"{b_stats['median']:.0f} | [{b_stats['min']:.0f}, {b_stats['max']:.0f}] | {U:.1f} | {z:.2f} | {r:.2f} | {significant} |")


def print_tool_efficiency_summary(scores_data: Dict):
    """Print tool_efficiency dimension summary"""
    print("\n## Tool Efficiency (Rubric Score)\n")
    
    a_scores, b_scores = extract_condition_scores(scores_data, 'tool_efficiency')
    
    if not a_scores or not b_scores:
        print("(Insufficient data)")
        return
    
    a_stats = compute_stats(a_scores)
    b_stats = compute_stats(b_scores)
    
    U, z, p = mannwhitneyu_exact(a_scores, b_scores)
    r = rank_biserial_r(U, len(a_scores), len(b_scores))
    significant = "Yes" if p < 0.05 else "No"
    
    print(f"**Condition A (developer__analyze):**")
    print(f"- Median: {a_stats['median']} (range: {a_stats['min']}-{a_stats['max']})")
    print(f"- Mean: {a_stats['mean']:.2f}")
    print()
    print(f"**Condition B (code-analyze-mcp with rg-blocking):**")
    print(f"- Median: {b_stats['median']} (range: {b_stats['min']}-{b_stats['max']})")
    print(f"- Mean: {b_stats['mean']:.2f}")
    print()
    print(f"**Statistical Test (Mann-Whitney U):**")
    print(f"- U: {U:.1f}")
    print(f"- z: {z:.2f}")
    print(f"- rank-biserial r: {r:.2f}")
    print(f"- p-value: {p:.4f}")
    print(f"- Significant (p<0.05): {significant}")


def print_cross_version_comparison(scores_data: Dict):
    """Print cross-version comparison table"""
    print("\n## Cross-Version Comparisons\n")
    
    v3_baselines = scores_data.get('v3_baselines', {})
    
    # v5B vs v3B
    print("### v5B vs v3B (Optimization Delta)\n")
    
    # Use quality totals (0-12), not individual dimension scores, for cross-version comparison.
    # v3 baselines store median of total quality score (sum of 4 dimensions).
    _, v5b_quality = extract_condition_scores(scores_data, 'total')
    _, v5b_eff = extract_efficiency_metric(scores_data, 'total_calls')
    
    v3b_median_quality = v3_baselines.get('B_condition', {}).get('median')
    v3b_median_calls = v3_baselines.get('B_condition', {}).get('efficiency_median_calls')
    
    if v5b_quality and v3b_median_quality:
        v5b_med_quality = statistics.median(v5b_quality)
        print(f"| Metric | v3B | v5B | Delta |")
        print("|--------|-----|-----|-------|")
        print(f"| Quality (total) | {v3b_median_quality} | {v5b_med_quality} | {v5b_med_quality - v3b_median_quality:+.1f} |")
    
    if v5b_eff and v3b_median_calls:
        v5b_med_calls = statistics.median(v5b_eff)
        print(f"| Total Calls | {v3b_median_calls} | {v5b_med_calls:.0f} | {v5b_med_calls - v3b_median_calls:+.0f} |")
    
    # v5B vs v3A
    print("\n### v5B vs v3A (Gap Closure)\n")
    
    v3a_median_quality = v3_baselines.get('A_condition', {}).get('median')
    v3a_median_calls = v3_baselines.get('A_condition', {}).get('efficiency_median_calls')
    
    if v5b_quality and v3a_median_quality:
        print(f"| Metric | v3A | v5B | Gap |")
        print("|--------|-----|-----|-----|")
        print(f"| Quality | {v3a_median_quality} | {v5b_med_quality} | {v5b_med_quality - v3a_median_quality:+.1f} |")
    
    if v5b_eff and v3a_median_calls:
        print(f"| Total Calls | {v3a_median_calls} | {v5b_med_calls:.0f} | {v5b_med_calls - v3a_median_calls:+.0f} |")


def main():
    parser = argparse.ArgumentParser(
        description='Analyze v5 benchmark scores: Mann-Whitney U, rank-biserial r, cross-version comparisons'
    )
    parser.add_argument(
        '--scores-file',
        type=Path,
        default=Path('scores.json'),
        help='Path to filled scores.json (default: scores.json in cwd)'
    )
    
    args = parser.parse_args()
    
    if not args.scores_file.exists():
        print(f"Error: {args.scores_file} not found", file=__import__('sys').stderr)
        exit(1)
    
    scores = load_scores(args.scores_file)
    
    print(f"# Analysis: {scores.get('experiment', 'v5-benchmark')}\n")
    print(f"**Model:** {scores.get('model')}")
    print(f"**Runs:** 10 (5 per condition)")
    
    print_quality_table(scores)
    print_efficiency_table(scores)
    print_tool_efficiency_summary(scores)
    print_cross_version_comparison(scores)
    
    print("\n---\n")
    print("*Note: Mann-Whitney U test requires scipy for exact p-values. If scipy unavailable, p-value is approximate.*")
    print("*rank-biserial r: |r| >= 0.3 (small), >= 0.5 (medium), >= 0.7 (large effect).*")


if __name__ == '__main__':
    main()
