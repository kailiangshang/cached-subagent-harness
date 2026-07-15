#!/usr/bin/env python3
"""Unit tests for the game-development A/B benchmark protocol."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parent))

import game_dev_ab_benchmark as bench


class GameDevAbBenchmarkTests(unittest.TestCase):
    def test_artifacts_include_identical_runnable_starters(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bench.write_artifacts(
                root,
                baseline_prompts=["base"],
                cached_prompts=["cached"],
            )
            baseline = root / "baseline-project"
            cached = root / "cached-harness-project"

            for relative in ("package.json", "index.html", "src/main.js"):
                self.assertEqual(
                    (baseline / relative).read_bytes(),
                    (cached / relative).read_bytes(),
                )

            package = json.loads(
                (baseline / "package.json").read_text(encoding="utf-8")
            )
            self.assertEqual(package["scripts"]["test"], "node --test")

    def test_starter_and_brief_fix_the_integration_contract(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            project = Path(tmp) / "project"
            bench.write_starter_project(project)
            main = (project / "src/main.js").read_text(encoding="utf-8")

        for contract in (
            'from "./game/engine.js"',
            'from "./ui/app.js"',
            'from "./session/records.js"',
            "createInitialState",
            "transition",
            "mountGame",
            "buildRunRecord",
        ):
            self.assertIn(contract, main)
        self.assertIn("Approved interface contract", bench.GAME_DEV_BRIEF)
        self.assertIn("Do not spawn or delegate nested agents", bench.GAME_DEV_BRIEF)
        self.assertIn("src/main.js", bench.worker_slice(3)["allowed_write_paths"])
        self.assertIn("index.html", bench.worker_slice(3)["allowed_write_paths"])
        self.assertIn("package.json", bench.worker_slice(3)["allowed_write_paths"])

    def test_artifact_regeneration_does_not_overwrite_developed_projects(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            arguments = {
                "baseline_prompts": ["base"],
                "cached_prompts": ["cached"],
            }
            bench.write_artifacts(root, **arguments)
            main = root / "baseline-project/src/main.js"
            main.write_text("// developed\n", encoding="utf-8")

            bench.write_artifacts(root, **arguments)

            self.assertEqual(main.read_text(encoding="utf-8"), "// developed\n")

    def test_cached_assignment_brief_names_the_exact_worker_slice(self) -> None:
        work_dir = Path("/tmp/signal-sweep-test")
        assignment = bench.build_cached_assignment_brief(work_dir, 1)
        expected = bench.worker_slice(1)

        self.assertIn(f"WORKER={expected['id']}", assignment)
        self.assertIn(f"SLICE={expected['title']}", assignment)
        self.assertIn(expected["task"], assignment)
        self.assertIn(f"QUALITY_GATE={expected['quality_gate']}", assignment)
        self.assertIn(str(work_dir / "signal-sweep-game-brief.md"), assignment)
        self.assertIn("Do not spawn or delegate nested agents", assignment)

    def test_break_even_prefers_cached_after_prefix_is_amortized(self) -> None:
        self.assertEqual(
            bench.break_even_dispatches(
                baseline_avg_tokens=500,
                stable_prefix_tokens_once=400,
                dynamic_tail_avg_tokens=100,
            ),
            2,
        )

    def test_break_even_can_be_unreachable(self) -> None:
        self.assertIsNone(
            bench.break_even_dispatches(
                baseline_avg_tokens=100,
                stable_prefix_tokens_once=400,
                dynamic_tail_avg_tokens=120,
            )
        )

    def test_report_tracks_quality_gates_and_status_observability(self) -> None:
        stable = "Stable harness operating contract.\n"
        harness_prompts = [
            f"{stable}\n{bench.DYNAMIC_MARKER}\nROLE=worker\nREPORT_PATH=/tmp/cached-{index}.md\n"
            for index in range(4)
        ]
        baseline_prompts = [
            "Self-contained game worker handoff.\n" + ("Full game spec. " * 180)
            for _ in range(4)
        ]

        report = bench.build_game_dev_report(
            harness_prompts=harness_prompts,
            baseline_prompts=baseline_prompts,
            workers=4,
            observations=[],
        )

        self.assertEqual(report["benchmark"], "game-dev-ab")
        self.assertEqual(report["workload"]["name"], "signal-sweep-browser-game")
        self.assertGreaterEqual(len(report["quality_gates"]), 3)
        self.assertIn("spawned", report["status_protocol"]["required_runtime_events"])
        self.assertIn("closed", report["status_protocol"]["required_runtime_events"])
        self.assertEqual(report["status_protocol"]["quality_gate_event"], "quality_passed")
        self.assertEqual(report["runs"]["baseline"]["prompt_count"], 4)
        self.assertEqual(report["runs"]["cached_harness"]["prompt_count"], 4)
        self.assertGreater(
            report["runs"]["cached_harness"]["stable_prefix_ratio_pct"],
            0,
        )
        self.assertGreater(
            report["savings"]["cache_adjusted_pct"],
            report["savings"]["raw_pct"],
        )

    def test_observations_are_aggregated_by_mode_and_worker(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "observations.jsonl"
            path.write_text(
                "\n".join(
                    [
                        '{"mode":"baseline","worker":"worker-01","event":"spawned"}',
                        '{"mode":"baseline","worker":"worker-01","event":"closed","usage_observed":true,"input_tokens":20,"cache_read_tokens":100,"output_tokens":25,"reasoning_tokens":5,"cache_write_tokens":0,"provider_input_tokens":120,"provider_output_tokens":30}',
                        '{"mode":"cached_harness","worker":"worker-01","event":"spawned"}',
                        '{"mode":"cached_harness","worker":"worker-01","event":"retry","usage_observed":true,"input_tokens":7,"cache_read_tokens":3,"output_tokens":4,"reasoning_tokens":1,"cache_write_tokens":0,"provider_input_tokens":10,"provider_output_tokens":5}',
                        '{"mode":"cached_harness","worker":"worker-01","event":"closed","usage_observed":true,"input_tokens":10,"cache_read_tokens":60,"output_tokens":20,"reasoning_tokens":5,"cache_write_tokens":0,"provider_input_tokens":70,"provider_output_tokens":25}',
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            observations = bench.load_observations(path)

        summary = bench.summarize_observations(observations, workers=1)
        self.assertEqual(summary["baseline"]["final_status"], "closed")
        self.assertEqual(summary["baseline"]["input_tokens_total"], 20)
        self.assertEqual(summary["baseline"]["cache_read_tokens_total"], 100)
        self.assertEqual(summary["baseline"]["provider_input_tokens_total"], 120)
        self.assertEqual(summary["baseline"]["total_effective_tokens"], 150)
        self.assertEqual(summary["cached_harness"]["final_status"], "closed")
        self.assertEqual(summary["cached_harness"]["input_tokens_total"], 17)
        self.assertEqual(summary["cached_harness"]["cache_read_tokens_total"], 63)
        self.assertEqual(summary["cached_harness"]["provider_input_tokens_total"], 80)
        self.assertEqual(summary["cached_harness"]["total_effective_tokens"], 110)
        self.assertEqual(summary["cached_harness"]["events_by_type"]["retry"], 1)

    def test_missing_usage_remains_unknown_instead_of_zero(self) -> None:
        summary = bench.summarize_observations(
            [
                {"mode": "baseline", "worker": "worker-01", "event": "closed"},
                {
                    "mode": "cached_harness",
                    "worker": "worker-01",
                    "event": "closed",
                    "usage_observed": True,
                    "input_tokens": 3,
                    "cache_read_tokens": 7,
                    "output_tokens": 2,
                    "reasoning_tokens": 1,
                    "cache_write_tokens": 0,
                    "provider_input_tokens": 10,
                    "provider_output_tokens": 3,
                },
            ],
            workers=1,
        )

        self.assertIsNone(summary["baseline"]["input_tokens_total"])
        self.assertIsNone(summary["baseline"]["total_effective_tokens"])
        self.assertEqual(summary["baseline"]["telemetry_quality"], "unknown")
        self.assertEqual(summary["cached_harness"]["telemetry_quality"], "exact")

    def test_codex_usage_is_split_into_non_overlapping_categories(self) -> None:
        normalized = bench.normalize_codex_usage(
            {
                "input_tokens": 1_000,
                "cached_input_tokens": 800,
                "output_tokens": 120,
                "reasoning_output_tokens": 20,
            }
        )

        self.assertEqual(
            normalized,
            {
                "usage_observed": True,
                "input_tokens": 200,
                "cache_read_tokens": 800,
                "output_tokens": 100,
                "reasoning_tokens": 20,
                "cache_write_tokens": 0,
                "provider_input_tokens": 1_000,
                "provider_output_tokens": 120,
                "telemetry_quality": "exact",
            },
        )

        partial = bench.normalize_codex_usage({"input_tokens": 100})
        self.assertIsNone(partial["input_tokens"])
        self.assertIsNone(partial["cache_read_tokens"])
        self.assertEqual(partial["provider_input_tokens"], 100)
        self.assertEqual(partial["telemetry_quality"], "partial")

    def test_observed_savings_stay_unknown_when_one_mode_lacks_usage(self) -> None:
        stable = "Stable harness contract.\n"
        harness_prompts = [
            f"{stable}\n{bench.DYNAMIC_MARKER}\nROLE=worker\nREPORT_PATH=/tmp/h-{index}.md\n"
            for index in range(2)
        ]
        baseline_prompts = ["Full baseline brief. " * 100 for _ in range(2)]
        usage = {
            "usage_observed": True,
            "input_tokens": 3,
            "cache_read_tokens": 7,
            "output_tokens": 2,
            "reasoning_tokens": 1,
            "cache_write_tokens": 0,
            "provider_input_tokens": 10,
            "provider_output_tokens": 3,
        }
        observations = [
            {
                "mode": "baseline",
                "worker": f"worker-{index:02d}",
                "event": "closed",
                **usage,
            }
            for index in (1, 2)
        ] + [
            {
                "mode": "cached_harness",
                "worker": f"worker-{index:02d}",
                "event": "closed",
            }
            for index in (1, 2)
        ]

        report = bench.build_game_dev_report(
            harness_prompts=harness_prompts,
            baseline_prompts=baseline_prompts,
            workers=2,
            observations=observations,
        )

        self.assertIsNone(
            report["savings"]["observed_runtime"]["provider_input_tokens_pct"]
        )
        self.assertIsNone(
            report["savings"]["observed_runtime"]["total_effective_tokens_pct"]
        )

    def test_observed_savings_require_equal_quality_gate_success(self) -> None:
        stable = "Stable harness contract.\n"
        prompts = [
            f"{stable}\n{bench.DYNAMIC_MARKER}\nROLE=worker\nREPORT_PATH=/tmp/{index}.md\n"
            for index in range(2)
        ]
        usage = {
            "usage_observed": True,
            "input_tokens": 3,
            "cache_read_tokens": 7,
            "output_tokens": 2,
            "reasoning_tokens": 1,
            "cache_write_tokens": 0,
            "provider_input_tokens": 10,
            "provider_output_tokens": 3,
        }
        observations = [
            {
                "mode": mode,
                "worker": f"worker-{index:02d}",
                "event": "closed",
                **usage,
            }
            for mode in bench.MODES
            for index in (1, 2)
        ]

        report = bench.build_game_dev_report(
            harness_prompts=prompts,
            baseline_prompts=["Full baseline brief. " * 100 for _ in range(2)],
            workers=2,
            observations=observations,
        )

        self.assertIsNone(
            report["savings"]["observed_runtime"]["provider_input_tokens_pct"]
        )
        self.assertFalse(report["savings"]["observed_runtime_comparable"])


if __name__ == "__main__":
    unittest.main()
