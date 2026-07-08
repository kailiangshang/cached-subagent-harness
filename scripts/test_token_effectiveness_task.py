#!/usr/bin/env python3
"""Unit tests for the offline token effectiveness task."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent))

import token_effectiveness_task as task


class TokenEffectivenessTaskTests(unittest.TestCase):
    def test_split_prompt_requires_dynamic_marker(self) -> None:
        with self.assertRaises(ValueError):
            task.split_prompt("no dynamic marker here")

    def test_estimate_tokens_uses_deterministic_byte_proxy(self) -> None:
        self.assertEqual(task.estimate_tokens(""), 0)
        self.assertEqual(task.estimate_tokens("abcd"), 1)
        self.assertEqual(task.estimate_tokens("abcde"), 2)

    def test_effectiveness_report_counts_stable_prefix_once(self) -> None:
        stable = "Stable operating rules.\nRole: worker\n"
        harness_prompts = [
            f"{stable}\n{task.DYNAMIC_MARKER}\nREPORT_PATH=/tmp/report-{index}.md\n"
            for index in range(4)
        ]
        long_embedded_brief = "Problem details. " * 120
        baseline_prompts = [
            f"Dispatch {index}\nFull brief:\n{long_embedded_brief}\nReport: /tmp/report-{index}.md\n"
            for index in range(4)
        ]

        report = task.build_effectiveness_report(
            harness_prompts=harness_prompts,
            baseline_prompts=baseline_prompts,
            agents=4,
            estimator="bytes/4",
        )

        self.assertLess(
            report["harness"]["cache_adjusted_estimated_tokens"],
            report["baseline"]["estimated_tokens_total"],
        )
        self.assertGreaterEqual(report["harness"]["stable_prefix_ratio_pct"], 40.0)
        self.assertGreaterEqual(report["savings"]["cache_adjusted_pct"], 50.0)


if __name__ == "__main__":
    unittest.main()
