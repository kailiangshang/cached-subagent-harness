#!/usr/bin/env python3
"""Offline token-effectiveness task for cached subagent dispatch prompts.

This script is intentionally provider-neutral. It does not call an LLM API or
claim billing accuracy; it uses a deterministic bytes/4 token proxy so CI can
catch prompt-shape regressions without credentials or network access.
"""

from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


DYNAMIC_MARKER = "--- DYNAMIC TASK CONTEXT ---"
DEFAULT_AGENTS = 4
DEFAULT_MIN_CACHE_ADJUSTED_SAVINGS_PCT = 50.0
DEFAULT_MIN_STABLE_PREFIX_RATIO_PCT = 55.0

REPRESENTATIVE_BRIEF = """Problem:
Refactor a feedback-agent platform so future database inspection, forum
inspection, user-reply workflows, session archival, push reports, code
association, admin permissions, and optimization advice can grow without
cross-feature regressions.

Scenarios:
1. A worker migrates one service boundary while src/ remains read-only.
2. An explorer maps repository/database/connector dependencies without edits.
3. A reviewer checks that the workflow still records memory, reports, and
   lifecycle state after migration.
4. A fixer addresses only Critical or Important findings from review.

Options:
1. Keep app.py as the center and patch features in place. Fast now, high
   coupling later.
2. Split ports, adapters, workflow state, memory, and reports behind harness
   gates. More structure, better parallel development.
3. Rewrite the whole application. High risk and hard to audit.

Chosen Plan:
Use the harness-controlled split. Give subagents paths, not pasted diffs or
logs. Keep PSOC, write scopes, ledgers, tests, review, and final audit as
non-optional gates.

Evidence to inspect by path:
- docs/migration-matrix.md
- reports/agent-ledger.db
- issue_feedback_agent/services/
- issue_feedback_agent/tests/
"""


def estimate_tokens(text: str) -> int:
    """Estimate tokens with a stable bytes/4 proxy."""
    if not text:
        return 0
    return math.ceil(len(text.encode("utf-8")) / 4)


def split_prompt(prompt: str) -> tuple[str, str]:
    """Split a harness prompt into stable prefix and dynamic tail."""
    marker_index = prompt.find(DYNAMIC_MARKER)
    if marker_index < 0:
        raise ValueError(f"missing dynamic marker: {DYNAMIC_MARKER}")
    stable = prompt[:marker_index].rstrip() + "\n"
    dynamic = prompt[marker_index:]
    return stable, dynamic


def pct(part: float, whole: float) -> float:
    if whole <= 0:
        return 0.0
    return round((part / whole) * 100.0, 2)


def savings_pct(before: int, after: int) -> float:
    if before <= 0:
        return 0.0
    return round(((before - after) / before) * 100.0, 2)


def longest_common_prefix(values: list[str]) -> str:
    if not values:
        return ""
    prefix = values[0]
    for value in values[1:]:
        limit = min(len(prefix), len(value))
        index = 0
        while index < limit and prefix[index] == value[index]:
            index += 1
        prefix = prefix[:index]
        if not prefix:
            break
    return prefix


def build_effectiveness_report(
    *,
    harness_prompts: list[str],
    baseline_prompts: list[str],
    agents: int,
    estimator: str,
) -> dict[str, Any]:
    if not harness_prompts:
        raise ValueError("at least one harness prompt is required")
    if not baseline_prompts:
        raise ValueError("at least one baseline prompt is required")

    split_prompts = [split_prompt(prompt) for prompt in harness_prompts]
    stable_parts = [stable for stable, _ in split_prompts]
    dynamic_parts = [dynamic for _, dynamic in split_prompts]
    stable_tokens = [estimate_tokens(stable) for stable in stable_parts]
    dynamic_tokens = [estimate_tokens(dynamic) for dynamic in dynamic_parts]
    harness_total = sum(estimate_tokens(prompt) for prompt in harness_prompts)
    baseline_total = sum(estimate_tokens(prompt) for prompt in baseline_prompts)

    stable_consistent = len(set(stable_parts)) == 1
    if stable_consistent:
        stable_once = stable_tokens[0]
        cache_adjusted = stable_once + sum(dynamic_tokens)
    else:
        common_prefix = longest_common_prefix(stable_parts)
        common_once = estimate_tokens(common_prefix)
        residual_total = 0
        for stable, dynamic in split_prompts:
            residual_total += estimate_tokens(stable[len(common_prefix) :] + dynamic)
        stable_once = common_once
        cache_adjusted = common_once + residual_total

    repeated_cacheable = max(0, sum(stable_tokens) - stable_once)
    average_stable_ratio = sum(
        pct(stable, stable + dynamic)
        for stable, dynamic in zip(stable_tokens, dynamic_tokens)
    ) / len(split_prompts)

    return {
        "estimator": estimator,
        "agents": agents,
        "baseline": {
            "estimated_tokens_total": baseline_total,
            "avg_tokens_per_prompt": round(baseline_total / len(baseline_prompts), 2),
            "prompt_count": len(baseline_prompts),
        },
        "harness": {
            "estimated_tokens_total": harness_total,
            "avg_tokens_per_prompt": round(harness_total / len(harness_prompts), 2),
            "cache_adjusted_estimated_tokens": cache_adjusted,
            "stable_prefix_estimated_tokens_once": stable_once,
            "dynamic_tail_estimated_tokens_total": sum(dynamic_tokens),
            "repeated_cacheable_estimated_tokens": repeated_cacheable,
            "stable_prefix_ratio_pct": round(average_stable_ratio, 2),
            "stable_prefix_consistent": stable_consistent,
            "prompt_count": len(harness_prompts),
        },
        "savings": {
            "raw_pct": savings_pct(baseline_total, harness_total),
            "cache_adjusted_pct": savings_pct(baseline_total, cache_adjusted),
        },
    }


def default_harnessctl_path() -> Path:
    repo = Path(__file__).resolve().parents[1]
    return repo / "skills" / "cached-subagent-harness" / "scripts" / "bin" / "harnessctl"


def run_harnessctl(harnessctl: Path, args: list[str]) -> str:
    result = subprocess.run(
        [str(harnessctl), *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())
    return result.stdout


def render_harness_prompts(harnessctl: Path, work_dir: Path, agents: int) -> list[str]:
    brief = work_dir / "feedback-agent-refactor-brief.md"
    ledger = work_dir / "agent-ledger.db"
    brief.write_text(REPRESENTATIVE_BRIEF, encoding="utf-8")

    prompts: list[str] = []
    for index in range(agents):
        report = work_dir / f"worker-{index + 1}-report.md"
        prompts.append(
            run_harnessctl(
                harnessctl,
                [
                    "render-prompt",
                    "--role",
                    "worker",
                    "--brief",
                    str(brief),
                    "--report",
                    str(report),
                    "--ledger",
                    str(ledger),
                    "--base-commit",
                    f"abc{index:04x}",
                    "--allowed-write-paths",
                    "issue_feedback_agent/services",
                    "--allowed-write-paths",
                    "issue_feedback_agent/tests",
                ],
            )
        )
    return prompts


def build_baseline_prompts(work_dir: Path, agents: int) -> list[str]:
    prompts: list[str] = []
    for index in range(agents):
        prompts.append(
            f"""Dispatch worker {index + 1}.
Branch: inspection-platform-agent
Base commit: abc{index:04x}
Report path: {work_dir / f'baseline-worker-{index + 1}-report.md'}
Allowed write paths: issue_feedback_agent/services, issue_feedback_agent/tests

Paste the full task brief into this subagent so it is self-contained:

{REPRESENTATIVE_BRIEF}

Recent controller notes:
- The user wants full development quality, not an MVP.
- Keep src/ read-only.
- Preserve memory workflow, report workflow, database inspection, and review gates.
- Current dispatch index {index + 1} must independently remember all context above.
"""
        )
    return prompts


def validate_report(
    report: dict[str, Any],
    *,
    min_raw_savings_pct: float | None,
    min_cache_adjusted_savings_pct: float,
    min_stable_prefix_ratio_pct: float,
) -> list[str]:
    errors: list[str] = []
    if (
        min_raw_savings_pct is not None
        and report["savings"]["raw_pct"] < min_raw_savings_pct
    ):
        errors.append(
            "raw savings "
            f"{report['savings']['raw_pct']}% below {min_raw_savings_pct}%"
        )
    if report["savings"]["cache_adjusted_pct"] < min_cache_adjusted_savings_pct:
        errors.append(
            "cache-adjusted savings "
            f"{report['savings']['cache_adjusted_pct']}% below "
            f"{min_cache_adjusted_savings_pct}%"
        )
    if report["harness"]["stable_prefix_ratio_pct"] < min_stable_prefix_ratio_pct:
        errors.append(
            "stable prefix ratio "
            f"{report['harness']['stable_prefix_ratio_pct']}% below "
            f"{min_stable_prefix_ratio_pct}%"
        )
    if not report["harness"]["stable_prefix_consistent"]:
        errors.append("harness stable prefix is not identical across prompts")
    return errors


def format_markdown(report: dict[str, Any]) -> str:
    return f"""# Token Effectiveness Task

Estimator: `{report['estimator']}`. This is an offline prompt-size proxy, not
provider billing telemetry.

| Metric | Baseline embedded handoff | Cached harness handoff |
|---|---:|---:|
| Prompt count | {report['baseline']['prompt_count']} | {report['harness']['prompt_count']} |
| Estimated tokens total | {report['baseline']['estimated_tokens_total']} | {report['harness']['estimated_tokens_total']} |
| Average tokens per prompt | {report['baseline']['avg_tokens_per_prompt']} | {report['harness']['avg_tokens_per_prompt']} |
| Cache-adjusted estimated tokens | n/a | {report['harness']['cache_adjusted_estimated_tokens']} |
| Stable prefix tokens counted once | n/a | {report['harness']['stable_prefix_estimated_tokens_once']} |
| Dynamic tail tokens total | n/a | {report['harness']['dynamic_tail_estimated_tokens_total']} |
| Repeated cacheable tokens | n/a | {report['harness']['repeated_cacheable_estimated_tokens']} |
| Stable prefix ratio | n/a | {report['harness']['stable_prefix_ratio_pct']}% |

Raw estimated savings: `{report['savings']['raw_pct']}%`

Cache-adjusted estimated savings: `{report['savings']['cache_adjusted_pct']}%`
"""


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--harnessctl",
        type=Path,
        default=default_harnessctl_path(),
        help="Path to the built harnessctl binary.",
    )
    parser.add_argument("--agents", type=int, default=DEFAULT_AGENTS)
    parser.add_argument("--output", type=Path, help="Optional report output path.")
    parser.add_argument(
        "--format",
        choices=("markdown", "json"),
        default="markdown",
        help="Output format.",
    )
    parser.add_argument(
        "--min-raw-savings-pct",
        type=float,
        default=None,
        help="Optional raw prompt-size gate. Omitted by default because stable prefixes may intentionally make single prompts larger.",
    )
    parser.add_argument(
        "--min-cache-adjusted-savings-pct",
        type=float,
        default=DEFAULT_MIN_CACHE_ADJUSTED_SAVINGS_PCT,
    )
    parser.add_argument(
        "--min-stable-prefix-ratio-pct",
        type=float,
        default=DEFAULT_MIN_STABLE_PREFIX_RATIO_PCT,
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv or sys.argv[1:])
    if args.agents < 2:
        raise SystemExit("--agents must be at least 2")
    if not args.harnessctl.is_file():
        raise SystemExit(
            f"missing harnessctl binary at {args.harnessctl}; run scripts/build-harnessctl.sh"
        )

    with tempfile.TemporaryDirectory(prefix="harness-token-task-") as tmp:
        work_dir = Path(tmp)
        harness_prompts = render_harness_prompts(args.harnessctl, work_dir, args.agents)
        baseline_prompts = build_baseline_prompts(work_dir, args.agents)
        report = build_effectiveness_report(
            harness_prompts=harness_prompts,
            baseline_prompts=baseline_prompts,
            agents=args.agents,
            estimator="bytes/4",
        )

    errors = validate_report(
        report,
        min_raw_savings_pct=args.min_raw_savings_pct,
        min_cache_adjusted_savings_pct=args.min_cache_adjusted_savings_pct,
        min_stable_prefix_ratio_pct=args.min_stable_prefix_ratio_pct,
    )
    rendered = (
        json.dumps(report, indent=2, sort_keys=True)
        if args.format == "json"
        else format_markdown(report)
    )
    if args.output:
        args.output.write_text(rendered + "\n", encoding="utf-8")
    else:
        print(rendered)

    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print("OK: token effectiveness thresholds passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
