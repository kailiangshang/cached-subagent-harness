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
                        '{"mode":"baseline","worker":"worker-01","event":"spawned","input_tokens":100}',
                        '{"mode":"baseline","worker":"worker-01","event":"closed","input_tokens":20,"output_tokens":30}',
                        '{"mode":"cached_harness","worker":"worker-01","event":"spawned","input_tokens":60}',
                        '{"mode":"cached_harness","worker":"worker-01","event":"closed","input_tokens":10,"output_tokens":25}',
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            observations = bench.load_observations(path)

        summary = bench.summarize_observations(observations, workers=1)
        self.assertEqual(summary["baseline"]["final_status"], "closed")
        self.assertEqual(summary["baseline"]["actual_input_tokens_total"], 120)
        self.assertEqual(summary["baseline"]["actual_output_tokens_total"], 30)
        self.assertEqual(summary["cached_harness"]["final_status"], "closed")
        self.assertEqual(summary["cached_harness"]["actual_input_tokens_total"], 70)
        self.assertEqual(summary["cached_harness"]["actual_output_tokens_total"], 25)


if __name__ == "__main__":
    unittest.main()
