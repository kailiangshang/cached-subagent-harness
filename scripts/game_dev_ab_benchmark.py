#!/usr/bin/env python3
"""Game-development A/B benchmark for cached subagent dispatch prompts.

The benchmark is provider-neutral. It generates equivalent worker prompts for a
small browser-game task in two modes:

- baseline: each worker receives a self-contained embedded handoff;
- cached_harness: each worker receives the harness stable prefix plus a dynamic
  tail that points to a shared brief and lifecycle ledger.

Optional JSONL observations can be supplied after a real run to aggregate
runtime status and token telemetry without making CI depend on a specific
agentic CLI.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
import tempfile
from pathlib import Path
from typing import Any

from token_effectiveness_task import (
    DYNAMIC_MARKER,
    build_effectiveness_report,
    default_harnessctl_path,
    run_harnessctl,
    savings_pct,
)


DEFAULT_WORKERS = 4
DEFAULT_MIN_CACHE_ADJUSTED_SAVINGS_PCT = 30.0
DEFAULT_MIN_STABLE_PREFIX_RATIO_PCT = 45.0
MODES = ("baseline", "cached_harness")
HARNESS_TOPOLOGIES = ("per-slice", "bounded-batch")
REQUIRED_RUNTIME_EVENTS = ("spawned", "running", "reported", "closed")
NORMALIZED_TOKEN_FIELDS = (
    "input_tokens",
    "cache_read_tokens",
    "output_tokens",
    "reasoning_tokens",
    "cache_write_tokens",
)
PROVIDER_TOKEN_FIELDS = ("provider_input_tokens", "provider_output_tokens")

QUALITY_GATES = [
    {
        "name": "engine-tests",
        "command": "npm test",
        "purpose": "Game rules, scoring, collision, restart, and timer behavior are covered.",
    },
    {
        "name": "build-or-static-smoke",
        "command": "npm run build || python3 -m http.server",
        "purpose": "The game can be served without missing assets or module errors.",
    },
    {
        "name": "desktop-mobile-screenshot",
        "command": "playwright screenshot at 1280x800 and 390x844",
        "purpose": "Canvas/UI is visible, framed, and not overlapped on desktop or mobile.",
    },
    {
        "name": "interaction-smoke",
        "command": "manual or scripted: start, move, pause, game-over, restart",
        "purpose": "Core loop is playable instead of only compiling.",
    },
]

GAME_DEV_BRIEF = """Problem:
Build a small browser arcade game called Signal Sweep. The player moves a
cursor through a 12x12 signal grid, collects pulses, avoids static traps, and
tries to beat a 60-second timer. The game must be playable as the first screen,
with no landing page or marketing copy.

Scenarios:
1. Engine worker implements deterministic grid generation, movement, scoring,
   timer, pause/resume, restart, and game-over state.
2. Rendering worker implements responsive canvas or DOM rendering with keyboard
   and touch controls, visible score/time/status, and no overlapping UI.
3. Persistence worker implements local high score, session summary, and an
   exportable JSON run record for later agent workflow tests.
4. Verification worker adds focused tests and a browser smoke checklist for
   desktop and mobile.

Options:
1. One worker receives the whole game. Lowest orchestration overhead, but weak
   status visibility and no parallelism.
2. Four self-contained workers each receive the full brief. Easy to dispatch,
   but repeats task context and lifecycle rules in every prompt.
3. Four workers share one brief through cached-subagent-harness. Higher control
   contract overhead per role, but task context stays in a shared file and
   dynamic tails remain small.

Chosen Plan:
Compare option 2 and option 3 with identical worker slices. Each worker must
record status events, report changed files, run the relevant quality gate, and
close its agent lifecycle entry. The benchmark treats raw prompt tokens,
cache-adjusted prompt tokens, runtime status, and observed token telemetry as
separate evidence.

Target implementation shape:
- Vanilla HTML/CSS/JavaScript unless the target repo already has a frontend
  framework.
- Stable dimensions for the board and controls.
- No decorative card-heavy landing page.
- Tests for pure game-state logic before UI wiring.
- Browser smoke evidence for desktop and mobile.

Approved interface contract (the design is complete; implement this contract
rather than redesigning it):
- `src/game/engine.js` exports `createInitialState(options)` and
  `transition(state, action)`. Actions cover `start`, `move`, `tick`, `pause`,
  and `restart`; `move` carries a direction and `tick` carries `elapsedMs`.
- `src/ui/app.js` exports `mountGame(root, { dispatch, onExport })` and returns
  an object with `render(state)` and `destroy()`.
- `src/session/records.js` exports `loadHighScore`, `saveHighScore`,
  `buildRunRecord`, and `downloadRunRecord`.
- `src/styles/game.css` owns the responsive game presentation.
- `src/main.js`, `index.html`, and `package.json` are fixed starter wiring. The
  integration worker may repair only their final cross-module wiring.
- Do not spawn or delegate nested agents.
"""

WORKER_SLICES = [
    {
        "id": "worker-01",
        "title": "engine",
        "task": "Implement game-state engine and focused tests for movement, scoring, traps, timer, pause, restart, and game-over.",
        "allowed_write_paths": ["src/game", "tests/game"],
        "quality_gate": "engine-tests",
    },
    {
        "id": "worker-02",
        "title": "rendering-controls",
        "task": "Implement responsive board rendering, keyboard controls, touch controls, score/time/status display, and layout constraints.",
        "allowed_write_paths": ["src/ui", "src/styles", "tests/ui"],
        "quality_gate": "desktop-mobile-screenshot",
    },
    {
        "id": "worker-03",
        "title": "session-records",
        "task": "Implement high-score persistence, session summary, and exportable JSON run records.",
        "allowed_write_paths": ["src/session", "tests/session"],
        "quality_gate": "build-or-static-smoke",
    },
    {
        "id": "worker-04",
        "title": "verification-integration",
        "task": "Wire integration tests, browser smoke checklist, and final playable workflow verification.",
        "allowed_write_paths": [
            "tests",
            "docs/benchmarks",
            "playwright.config.js",
            "src/main.js",
            "index.html",
            "package.json",
        ],
        "quality_gate": "interaction-smoke",
    },
]

STARTER_INDEX = """<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="dark">
  <title>Signal Sweep</title>
  <link rel="stylesheet" href="./src/styles/game.css">
  <script type="module" src="./src/main.js"></script>
</head>
<body>
  <main id="game" aria-label="Signal Sweep game" aria-live="polite"></main>
  <noscript>Signal Sweep requires JavaScript.</noscript>
</body>
</html>
"""

STARTER_MAIN = """import { createInitialState, transition } from "./game/engine.js";
import { mountGame } from "./ui/app.js";
import {
  buildRunRecord,
  downloadRunRecord,
  loadHighScore,
  saveHighScore,
} from "./session/records.js";

const TICK_MS = 250;
const root = document.querySelector("#game");

if (!root) throw new Error("Missing #game root");

let savedHighScore = loadHighScore();
let state = createInitialState({ highScore: savedHighScore });
let view;

function dispatch(action) {
  state = transition(state, action);
  const nextHighScore = Math.max(savedHighScore, Number(state.score) || 0);
  if (nextHighScore > savedHighScore) {
    savedHighScore = nextHighScore;
    saveHighScore(savedHighScore);
  }
  view.render(state);
}

view = mountGame(root, {
  dispatch,
  onExport: () => downloadRunRecord(buildRunRecord(state)),
});
view.render(state);

window.setInterval(() => dispatch({ type: "tick", elapsedMs: TICK_MS }), TICK_MS);
"""


def worker_slice(index: int) -> dict[str, Any]:
    if index < len(WORKER_SLICES):
        return WORKER_SLICES[index]
    worker_id = f"worker-{index + 1:02d}"
    return {
        "id": worker_id,
        "title": f"extended-verification-{index + 1}",
        "task": "Run an additional focused verification pass without changing unrelated files.",
        "allowed_write_paths": ["tests", "docs/benchmarks"],
        "quality_gate": "interaction-smoke",
    }


def break_even_dispatches(
    *,
    baseline_avg_tokens: float,
    stable_prefix_tokens_once: float,
    dynamic_tail_avg_tokens: float,
) -> int | None:
    """Return the first dispatch count where cached cost is strictly lower."""
    per_dispatch_delta = baseline_avg_tokens - dynamic_tail_avg_tokens
    if per_dispatch_delta <= 0:
        return None
    return math.floor(stable_prefix_tokens_once / per_dispatch_delta) + 1


def build_baseline_prompt(work_dir: Path, index: int) -> str:
    slice_info = worker_slice(index)
    return f"""Dispatch Signal Sweep baseline worker.
MODE=baseline
WORKER={slice_info['id']}
SLICE={slice_info['title']}
REPORT_PATH={work_dir / f"baseline-{slice_info['id']}-report.md"}
STATUS_LOG_PATH={work_dir / "baseline-status-observations.jsonl"}
ALLOWED_WRITE_PATHS={",".join(slice_info['allowed_write_paths'])}
QUALITY_GATE={slice_info['quality_gate']}

You are not alone in the codebase. Do not revert unrelated edits. Write only in
the allowed paths above. Record runtime status events when actually executing:
spawned, running, reported, closed. Include input/output token telemetry when
the CLI exposes it.

Worker task:
{slice_info['task']}

Full shared task brief pasted for self-contained baseline dispatch:

{GAME_DEV_BRIEF}

Required final report:
- files changed;
- tests or smoke checks run;
- current worker status;
- blockers and residual risk;
- confirmation that the worker agent was closed or a reason it was not closed.
"""


def build_baseline_prompts(work_dir: Path, workers: int) -> list[str]:
    return [build_baseline_prompt(work_dir, index) for index in range(workers)]


def build_cached_assignment_brief(work_dir: Path, index: int) -> str:
    slice_info = worker_slice(index)
    return f"""Worker assignment for the approved Signal Sweep design.
WORKER={slice_info['id']}
SLICE={slice_info['title']}
SHARED_BRIEF_PATH={work_dir / "signal-sweep-game-brief.md"}
ALLOWED_WRITE_PATHS={",".join(slice_info['allowed_write_paths'])}
QUALITY_GATE={slice_info['quality_gate']}

Read SHARED_BRIEF_PATH before editing, then complete exactly this slice:
{slice_info['task']}

Follow the shared interface contract. Do not spawn or delegate nested agents.
Write the required report and stay inside ALLOWED_WRITE_PATHS.
"""


def _batch_write_scope(workers: int) -> list[str]:
    return sorted(
        {
            allowed_path
            for index in range(workers)
            for allowed_path in worker_slice(index)["allowed_write_paths"]
        }
    )


def build_cached_batch_brief(work_dir: Path, workers: int) -> str:
    slices = [worker_slice(index) for index in range(workers)]
    skill_path = (
        Path(__file__).resolve().parents[1]
        / "skills"
        / "cached-subagent-harness"
        / "SKILL.md"
    )
    slice_text = "\n\n".join(
        "\n".join(
            [
                f"{index + 1}. WORKER={slice_info['id']}",
                f"   SLICE={slice_info['title']}",
                f"   TASK={slice_info['task']}",
                f"   QUALITY_GATE={slice_info['quality_gate']}",
                "   ALLOWED_WRITE_PATHS="
                + ",".join(slice_info["allowed_write_paths"]),
            ]
        )
        for index, slice_info in enumerate(slices)
    )
    return f"""Bounded worker batch for the approved Signal Sweep design.
BATCH_ID=signal-sweep-batch-01
SESSION_TOPOLOGY=one-fresh-session-zero-followups
SKILL_PATH={skill_path}
SHARED_BRIEF_PATH={work_dir / "signal-sweep-game-brief.md"}
REPORT_PATH={work_dir / "cached-batch-01-report.md"}
ALLOWED_WRITE_PATHS={",".join(_batch_write_scope(workers))}

Read SKILL_PATH and SHARED_BRIEF_PATH before editing. Complete these ordered
slices inside one bounded assignment and one final review boundary:

{slice_text}

Use TDD within each slice, then run the complete project gates once.
Do not spawn or delegate nested agents. Do not resume or request a follow-up.
Write one dense report covering every slice and stay inside ALLOWED_WRITE_PATHS.
"""


def render_cached_prompts(harnessctl: Path, work_dir: Path, workers: int) -> list[str]:
    shared_brief = work_dir / "signal-sweep-game-brief.md"
    ledger = work_dir / "cached-harness-agent-ledger.db"
    shared_brief.write_text(GAME_DEV_BRIEF, encoding="utf-8")

    prompts: list[str] = []
    for index in range(workers):
        slice_info = worker_slice(index)
        assignment_brief = work_dir / f"cached-{slice_info['id']}-brief.md"
        assignment_brief.write_text(
            build_cached_assignment_brief(work_dir, index), encoding="utf-8"
        )
        args = [
            "render-prompt",
            "--role",
            "worker",
            "--brief",
            str(assignment_brief),
            "--report",
            str(work_dir / f"cached-{slice_info['id']}-report.md"),
            "--ledger",
            str(ledger),
            "--base-commit",
            "HEAD",
            "--context",
            str(shared_brief),
        ]
        for allowed_path in slice_info["allowed_write_paths"]:
            args.extend(["--allowed-write-paths", allowed_path])
        prompts.append(run_harnessctl(harnessctl, args))
    return prompts


def render_cached_batch_prompt(
    harnessctl: Path, work_dir: Path, workers: int
) -> str:
    shared_brief = work_dir / "signal-sweep-game-brief.md"
    assignment_brief = work_dir / "cached-batch-01-brief.md"
    ledger = work_dir / "cached-harness-agent-ledger.db"
    skill_path = (
        Path(__file__).resolve().parents[1]
        / "skills"
        / "cached-subagent-harness"
        / "SKILL.md"
    )
    shared_brief.write_text(GAME_DEV_BRIEF, encoding="utf-8")
    assignment_brief.write_text(
        build_cached_batch_brief(work_dir, workers), encoding="utf-8"
    )
    args = [
        "render-prompt",
        "--role",
        "worker",
        "--brief",
        str(assignment_brief),
        "--report",
        str(work_dir / "cached-batch-01-report.md"),
        "--ledger",
        str(ledger),
        "--base-commit",
        "HEAD",
        "--context",
        str(skill_path),
        "--context",
        str(shared_brief),
        "--harness-command",
        "npm test",
    ]
    for allowed_path in _batch_write_scope(workers):
        args.extend(["--allowed-write-paths", allowed_path])
    return run_harnessctl(harnessctl, args)


def load_observations(path: Path) -> list[dict[str, Any]]:
    observations: list[dict[str, Any]] = []
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line:
            continue
        try:
            observation = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{line_number}: invalid JSON: {error}") from error
        if not isinstance(observation, dict):
            raise ValueError(f"{path}:{line_number}: observation must be a JSON object")
        mode = observation.get("mode")
        if mode not in MODES:
            raise ValueError(f"{path}:{line_number}: unknown mode {mode!r}")
        if "worker" not in observation or "event" not in observation:
            raise ValueError(f"{path}:{line_number}: worker and event are required")
        observations.append(observation)
    return observations


def _optional_token(value: Any, name: str) -> int | None:
    if value is None:
        return None
    if isinstance(value, bool):
        raise ValueError(f"{name} must be a nonnegative integer")
    try:
        number = int(value)
    except (TypeError, ValueError) as error:
        raise ValueError(f"{name} must be a nonnegative integer") from error
    if number < 0 or str(number) != str(value):
        raise ValueError(f"{name} must be a canonical nonnegative integer")
    return number


def normalize_codex_usage(usage: dict[str, Any]) -> dict[str, Any]:
    """Split Codex turn totals into non-overlapping harness categories."""
    provider_input = _optional_token(usage.get("input_tokens"), "input_tokens")
    cached_input = _optional_token(
        usage.get("cached_input_tokens"), "cached_input_tokens"
    )
    provider_output = _optional_token(usage.get("output_tokens"), "output_tokens")
    reasoning_output = _optional_token(
        usage.get("reasoning_output_tokens"), "reasoning_output_tokens"
    )
    observed = any(
        value is not None
        for value in (
            provider_input,
            cached_input,
            provider_output,
            reasoning_output,
        )
    )
    if (
        provider_input is not None
        and cached_input is not None
        and cached_input > provider_input
    ):
        raise ValueError("cached_input_tokens exceeds input_tokens")
    if (
        provider_output is not None
        and reasoning_output is not None
        and reasoning_output > provider_output
    ):
        raise ValueError("reasoning_output_tokens exceeds output_tokens")

    complete = all(
        value is not None
        for value in (
            provider_input,
            cached_input,
            provider_output,
            reasoning_output,
        )
    )
    return {
        "usage_observed": observed,
        "input_tokens": (
            provider_input - cached_input
            if provider_input is not None and cached_input is not None
            else None
        ),
        "cache_read_tokens": cached_input,
        "output_tokens": (
            provider_output - reasoning_output
            if provider_output is not None and reasoning_output is not None
            else None
        ),
        "reasoning_tokens": reasoning_output,
        # Codex exposes cached reads inside provider input, not a separate
        # cache-write counter. Zero means no additional category is added.
        "cache_write_tokens": 0 if observed else None,
        "provider_input_tokens": provider_input,
        "provider_output_tokens": provider_output,
        "telemetry_quality": "exact" if complete else ("partial" if observed else "unknown"),
    }


def _complete_sum(rows: list[dict[str, Any]], field: str, coverage: bool) -> int | None:
    if not rows or not coverage:
        return None
    values = [_optional_token(row.get(field), field) for row in rows]
    if any(value is None for value in values):
        return None
    return sum(value for value in values if value is not None)


def _expected_worker_ids(workers: int) -> tuple[str, ...]:
    if workers < 1 or workers > len(WORKER_SLICES):
        raise ValueError(f"workers must be between 1 and {len(WORKER_SLICES)}")
    return tuple(worker["id"] for worker in WORKER_SLICES[:workers])


def per_slice_runtime_topology(workers: int) -> dict[str, dict[str, Any]]:
    units = list(_expected_worker_ids(workers))
    return {
        "baseline": {
            "strategy": "fresh_per_slice",
            "units": units,
            "assignment_count": workers,
            "session_count": workers,
            "followup_count": 0,
        },
        "cached_harness": {
            "strategy": "harness_per_slice",
            "units": list(units),
            "assignment_count": workers,
            "session_count": workers,
            "followup_count": 0,
        },
    }


def corrected_runtime_topology(workers: int) -> dict[str, dict[str, Any]]:
    topology = per_slice_runtime_topology(workers)
    topology["cached_harness"] = {
        "strategy": "bounded_batch",
        "units": ["batch-01"],
        "assignment_count": workers,
        "session_count": 1,
        "followup_count": 0,
    }
    return topology


def _validated_runtime_topology(
    workers: int,
    runtime_topology: dict[str, dict[str, Any]] | None,
) -> dict[str, dict[str, Any]]:
    topology = runtime_topology or per_slice_runtime_topology(workers)
    if set(topology) != set(MODES):
        raise ValueError(f"runtime topology must define exactly {MODES}")
    for mode in MODES:
        facts = topology[mode]
        units = facts.get("units")
        if (
            not isinstance(units, list)
            or not units
            or any(not isinstance(unit, str) or not unit for unit in units)
            or len(set(units)) != len(units)
        ):
            raise ValueError(f"runtime topology has invalid units for {mode}")
        for field in ("assignment_count", "session_count", "followup_count"):
            value = facts.get(field)
            if isinstance(value, bool) or not isinstance(value, int) or value < 0:
                raise ValueError(f"runtime topology has invalid {mode}.{field}")
        if facts["assignment_count"] != workers:
            raise ValueError(
                f"runtime topology {mode}.assignment_count must equal workers"
            )
    return topology


def _usage_observation_is_exact(row: dict[str, Any]) -> bool:
    if row.get("usage_observed") is not True:
        return False
    quality = str(row.get("telemetry_quality", "unknown"))
    if quality not in {"exact", "partial", "estimated", "unsupported", "unknown"}:
        raise ValueError(f"invalid telemetry_quality: {quality}")
    values = {
        field: _optional_token(row.get(field), field)
        for field in (*NORMALIZED_TOKEN_FIELDS, *PROVIDER_TOKEN_FIELDS)
    }
    complete = all(value is not None for value in values.values())
    if quality == "exact" and not complete:
        raise ValueError("exact telemetry row is missing required token fields")
    if complete:
        if (
            values["input_tokens"] + values["cache_read_tokens"]
            != values["provider_input_tokens"]
        ):
            raise ValueError(
                "input_tokens + cache_read_tokens must equal provider_input_tokens"
            )
        if (
            values["output_tokens"] + values["reasoning_tokens"]
            != values["provider_output_tokens"]
        ):
            raise ValueError(
                "output_tokens + reasoning_tokens must equal provider_output_tokens"
            )
    return quality == "exact" and complete


def summarize_observations(
    observations: list[dict[str, Any]],
    *,
    workers: int,
    runtime_topology: dict[str, dict[str, Any]] | None = None,
) -> dict[str, dict[str, Any]]:
    topology = _validated_runtime_topology(workers, runtime_topology)
    expected_workers_by_mode = {
        mode: set(topology[mode]["units"]) for mode in MODES
    }
    expected_quality_gates = {gate["name"] for gate in QUALITY_GATES}
    summary: dict[str, dict[str, Any]] = {
        mode: {
            "event_count": 0,
            "execution_units_seen": 0,
            "execution_units_closed": 0,
            "workers_seen": 0,
            "workers_closed": 0,
            "final_status": "not-observed",
            "quality_gates_passed": False,
            "quality_gates_seen": [],
            "usage_observation_count": 0,
            "comparable_usage_observation_count": 0,
            "retry_usage_observation_count": 0,
            "execution_units_with_usage": 0,
            "workers_with_usage": 0,
            "telemetry_quality": "unknown",
            "comparable_telemetry_quality": "unknown",
            "retry_telemetry_quality": "unknown",
            "input_tokens_total": None,
            "cache_read_tokens_total": None,
            "output_tokens_total": None,
            "reasoning_tokens_total": None,
            "cache_write_tokens_total": None,
            "provider_input_tokens_total": None,
            "provider_output_tokens_total": None,
            "total_effective_tokens": None,
            "comparable_total_effective_tokens": None,
            "retry_total_effective_tokens": None,
            "events_by_type": {},
        }
        for mode in MODES
    }
    workers_by_mode: dict[str, set[str]] = {mode: set() for mode in MODES}
    lifecycle_by_mode: dict[str, dict[str, set[str]]] = {
        mode: {worker: set() for worker in expected_workers_by_mode[mode]}
        for mode in MODES
    }
    latest_event_by_worker: dict[str, dict[str, str]] = {mode: {} for mode in MODES}
    usage_rows_by_mode: dict[str, list[dict[str, Any]]] = {
        mode: [] for mode in MODES
    }
    usage_rows_exact_by_mode: dict[str, list[bool]] = {mode: [] for mode in MODES}
    comparable_rows_by_mode: dict[str, list[dict[str, Any]]] = {
        mode: [] for mode in MODES
    }
    comparable_workers_by_mode: dict[str, set[str]] = {
        mode: set() for mode in MODES
    }
    comparable_rows_exact_by_mode: dict[str, list[bool]] = {
        mode: [] for mode in MODES
    }
    retry_rows_by_mode: dict[str, list[dict[str, Any]]] = {
        mode: [] for mode in MODES
    }
    retry_rows_exact_by_mode: dict[str, list[bool]] = {
        mode: [] for mode in MODES
    }
    quality_gates_by_mode: dict[str, set[str]] = {mode: set() for mode in MODES}

    for observation in observations:
        mode = str(observation["mode"])
        worker = str(observation["worker"])
        event = str(observation["event"])
        if mode not in MODES:
            raise ValueError(f"invalid observation mode: {mode}")
        expected_workers = expected_workers_by_mode[mode]
        if worker not in expected_workers:
            raise ValueError(f"unknown worker for {mode} topology: {worker}")
        mode_summary = summary[mode]

        mode_summary["event_count"] += 1
        events_by_type = mode_summary["events_by_type"]
        events_by_type[event] = events_by_type.get(event, 0) + 1
        if event == "quality_passed":
            quality_gate = str(observation.get("quality_gate", ""))
            if quality_gate not in expected_quality_gates:
                raise ValueError(f"invalid or missing quality_gate: {quality_gate}")
            if quality_gate in quality_gates_by_mode[mode]:
                raise ValueError(f"duplicate quality gate event: {mode}/{quality_gate}")
            quality_gates_by_mode[mode].add(quality_gate)
        workers_by_mode[mode].add(worker)
        if event in REQUIRED_RUNTIME_EVENTS:
            if event in lifecycle_by_mode[mode][worker]:
                raise ValueError(f"duplicate lifecycle event: {mode}/{worker}/{event}")
            lifecycle_by_mode[mode][worker].add(event)
            latest_event_by_worker[mode][worker] = event
        if observation.get("usage_observed") is True:
            if event not in {"closed", "retry"}:
                raise ValueError(
                    "usage is only supported on closed or retry events: "
                    f"{mode}/{worker}/{event}"
                )
            exact = _usage_observation_is_exact(observation)
            usage_rows_by_mode[mode].append(observation)
            usage_rows_exact_by_mode[mode].append(exact)
            if event == "closed":
                comparable_rows_by_mode[mode].append(observation)
                comparable_rows_exact_by_mode[mode].append(exact)
                if exact:
                    comparable_workers_by_mode[mode].add(worker)
            elif event == "retry":
                retry_rows_by_mode[mode].append(observation)
                retry_rows_exact_by_mode[mode].append(exact)

    for mode in MODES:
        expected_workers = expected_workers_by_mode[mode]
        mode_summary = summary[mode]
        execution_units_seen = len(workers_by_mode[mode])
        execution_units_closed = sum(
            1
            for worker in expected_workers
            if "closed" in lifecycle_by_mode[mode][worker]
        )
        mode_summary["execution_units_seen"] = execution_units_seen
        mode_summary["execution_units_closed"] = execution_units_closed
        # Compatibility aliases for older report consumers.
        mode_summary["workers_seen"] = execution_units_seen
        mode_summary["workers_closed"] = execution_units_closed
        quality_gates_seen = quality_gates_by_mode[mode]
        mode_summary["quality_gates_seen"] = [
            gate["name"] for gate in QUALITY_GATES if gate["name"] in quality_gates_seen
        ]
        mode_summary["quality_gates_passed"] = (
            quality_gates_seen == expected_quality_gates
        )
        usage_rows = usage_rows_by_mode[mode]
        comparable_rows = comparable_rows_by_mode[mode]
        retry_rows = retry_rows_by_mode[mode]
        comparable_coverage = (
            comparable_workers_by_mode[mode] == expected_workers
            and len(comparable_rows) == len(expected_workers)
            and all(comparable_rows_exact_by_mode[mode])
        )
        retry_event_count = mode_summary["events_by_type"].get("retry", 0)
        retry_coverage = retry_event_count == len(retry_rows) and all(
            retry_rows_exact_by_mode[mode]
        )
        coverage = (
            comparable_coverage
            and retry_coverage
            and all(usage_rows_exact_by_mode[mode])
        )
        mode_summary["usage_observation_count"] = len(usage_rows)
        mode_summary["comparable_usage_observation_count"] = len(comparable_rows)
        mode_summary["retry_usage_observation_count"] = len(retry_rows)
        execution_units_with_usage = len(comparable_workers_by_mode[mode])
        mode_summary["execution_units_with_usage"] = execution_units_with_usage
        mode_summary["workers_with_usage"] = execution_units_with_usage
        for field in (*NORMALIZED_TOKEN_FIELDS, *PROVIDER_TOKEN_FIELDS):
            total_name = f"{field}_total"
            mode_summary[total_name] = _complete_sum(usage_rows, field, coverage)
            mode_summary[f"comparable_{total_name}"] = _complete_sum(
                comparable_rows, field, comparable_coverage
            )
            mode_summary[f"retry_{total_name}"] = (
                0
                if retry_event_count == 0
                else _complete_sum(retry_rows, field, retry_coverage)
            )
        normalized_totals = [
            mode_summary[f"{field}_total"] for field in NORMALIZED_TOKEN_FIELDS
        ]
        comparable_normalized_totals = [
            mode_summary[f"comparable_{field}_total"]
            for field in NORMALIZED_TOKEN_FIELDS
        ]
        retry_normalized_totals = [
            mode_summary[f"retry_{field}_total"]
            for field in NORMALIZED_TOKEN_FIELDS
        ]
        if all(value is not None for value in normalized_totals):
            mode_summary["total_effective_tokens"] = sum(normalized_totals)
            mode_summary["telemetry_quality"] = "exact"
        elif usage_rows:
            mode_summary["telemetry_quality"] = "partial"
        if all(value is not None for value in comparable_normalized_totals):
            mode_summary["comparable_total_effective_tokens"] = sum(
                comparable_normalized_totals
            )
            mode_summary["comparable_telemetry_quality"] = "exact"
        elif comparable_rows:
            mode_summary["comparable_telemetry_quality"] = "partial"
        if all(value is not None for value in retry_normalized_totals):
            mode_summary["retry_total_effective_tokens"] = sum(
                retry_normalized_totals
            )
            mode_summary["retry_telemetry_quality"] = "exact"
        elif retry_event_count:
            mode_summary["retry_telemetry_quality"] = "partial"
        if mode_summary["event_count"] == 0:
            mode_summary["final_status"] = "not-observed"
        elif all(
            set(REQUIRED_RUNTIME_EVENTS).issubset(lifecycle_by_mode[mode][worker])
            and latest_event_by_worker[mode].get(worker) == "closed"
            for worker in expected_workers
        ):
            mode_summary["final_status"] = "closed"
        elif mode_summary["workers_seen"] >= len(expected_workers):
            mode_summary["final_status"] = "partial"
        else:
            mode_summary["final_status"] = "incomplete"
    return summary


def build_game_dev_report(
    *,
    harness_prompts: list[str],
    baseline_prompts: list[str],
    workers: int,
    observations: list[dict[str, Any]],
    runtime_topology: dict[str, dict[str, Any]] | None = None,
) -> dict[str, Any]:
    topology = _validated_runtime_topology(workers, runtime_topology)
    effectiveness = build_effectiveness_report(
        harness_prompts=harness_prompts,
        baseline_prompts=baseline_prompts,
        agents=workers,
        estimator="bytes/4",
    )
    dynamic_avg = (
        effectiveness["harness"]["dynamic_tail_estimated_tokens_total"]
        / len(harness_prompts)
    )
    break_even = break_even_dispatches(
        baseline_avg_tokens=effectiveness["baseline"]["avg_tokens_per_prompt"],
        stable_prefix_tokens_once=effectiveness["harness"][
            "stable_prefix_estimated_tokens_once"
        ],
        dynamic_tail_avg_tokens=dynamic_avg,
    )
    observation_summary = summarize_observations(
        observations,
        workers=workers,
        runtime_topology=topology,
    )
    observed_savings: dict[str, float | None] = {
        "provider_input_tokens_pct": None,
        "total_effective_tokens_pct": None,
    }
    comparable_sample_savings: dict[str, float | None] = {
        "provider_input_tokens_pct": None,
        "total_effective_tokens_pct": None,
    }
    baseline_observed = observation_summary["baseline"]
    cached_observed = observation_summary["cached_harness"]
    comparable_sample_valid = (
        baseline_observed["final_status"] == "closed"
        and cached_observed["final_status"] == "closed"
        and baseline_observed["quality_gates_passed"]
        and cached_observed["quality_gates_passed"]
        and baseline_observed["comparable_telemetry_quality"] == "exact"
        and cached_observed["comparable_telemetry_quality"] == "exact"
    )
    observed_runtime_comparable = (
        comparable_sample_valid
        and baseline_observed["telemetry_quality"] == "exact"
        and cached_observed["telemetry_quality"] == "exact"
    )
    if comparable_sample_valid and (
        baseline_observed["comparable_provider_input_tokens_total"]
        and cached_observed["comparable_provider_input_tokens_total"] is not None
    ):
        comparable_sample_savings["provider_input_tokens_pct"] = savings_pct(
            baseline_observed["comparable_provider_input_tokens_total"],
            cached_observed["comparable_provider_input_tokens_total"],
        )
    if comparable_sample_valid and (
        baseline_observed["comparable_total_effective_tokens"]
        and cached_observed["comparable_total_effective_tokens"] is not None
    ):
        comparable_sample_savings["total_effective_tokens_pct"] = savings_pct(
            baseline_observed["comparable_total_effective_tokens"],
            cached_observed["comparable_total_effective_tokens"],
        )
    if observed_runtime_comparable and (
        baseline_observed["provider_input_tokens_total"]
        and cached_observed["provider_input_tokens_total"] is not None
    ):
        observed_savings["provider_input_tokens_pct"] = savings_pct(
            baseline_observed["provider_input_tokens_total"],
            cached_observed["provider_input_tokens_total"],
        )
    if observed_runtime_comparable and (
        baseline_observed["total_effective_tokens"]
        and cached_observed["total_effective_tokens"] is not None
    ):
        observed_savings["total_effective_tokens_pct"] = savings_pct(
            baseline_observed["total_effective_tokens"],
            cached_observed["total_effective_tokens"],
        )

    return {
        "benchmark": "game-dev-ab",
        "estimator": "bytes/4",
        "workload": {
            "name": "signal-sweep-browser-game",
            "workers": workers,
            "description": "Equivalent four-slice browser-game development task.",
        },
        "quality_gates": QUALITY_GATES,
        "runtime_topology": topology,
        "status_protocol": {
            "required_runtime_events": list(REQUIRED_RUNTIME_EVENTS),
            "quality_gate_event": "quality_passed",
            "observation_jsonl_fields": [
                "mode",
                "worker",
                "event",
                "input_tokens",
                "cache_read_tokens",
                "output_tokens",
                "reasoning_tokens",
                "cache_write_tokens",
                "provider_input_tokens",
                "provider_output_tokens",
                "usage_observed",
                "telemetry_quality",
                "quality_gate",
                "elapsed_ms",
                "note",
            ],
            "offline_events_are_artifact_status_only": True,
        },
        "runs": {
            "baseline": {
                **effectiveness["baseline"],
                "cache_adjusted_estimated_tokens": effectiveness["baseline"][
                    "estimated_tokens_total"
                ],
                "mode_description": "Full task brief and status instructions embedded in every worker prompt.",
            },
            "cached_harness": {
                **effectiveness["harness"],
                "mode_description": "Stable harness prefix plus dynamic paths to the shared game brief and ledger.",
            },
        },
        "savings": {
            **effectiveness["savings"],
            "break_even_dispatches": break_even,
            "observed_runtime": observed_savings,
            "observed_runtime_comparable_sample": comparable_sample_savings,
            "observed_runtime_comparable": observed_runtime_comparable,
            "comparable_sample_valid": comparable_sample_valid,
        },
        "status_observations": observation_summary,
        "interpretation": {
            "raw_tokens": "Prompt bytes sent before provider prompt-cache effects.",
            "cache_adjusted_tokens": "Stable harness prefix counted once, dynamic tails counted per dispatch.",
            "runtime_status": "Only populated from an external observations JSONL after actual subagents run.",
            "comparable_sample": "Closed execution-unit usage only; excludes retry attempts.",
            "operational_total": "All exact closed and retry usage; retry-inclusive telemetry must be complete.",
        },
    }


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
            f"raw savings {report['savings']['raw_pct']}% below {min_raw_savings_pct}%"
        )
    if report["savings"]["cache_adjusted_pct"] < min_cache_adjusted_savings_pct:
        errors.append(
            "cache-adjusted savings "
            f"{report['savings']['cache_adjusted_pct']}% below "
            f"{min_cache_adjusted_savings_pct}%"
        )
    if (
        report["runs"]["cached_harness"]["stable_prefix_ratio_pct"]
        < min_stable_prefix_ratio_pct
    ):
        errors.append(
            "stable prefix ratio "
            f"{report['runs']['cached_harness']['stable_prefix_ratio_pct']}% below "
            f"{min_stable_prefix_ratio_pct}%"
        )
    if not report["runs"]["cached_harness"]["stable_prefix_consistent"]:
        errors.append("cached harness stable prefix is not identical across prompts")
    return errors


def write_starter_project(project_dir: Path) -> None:
    """Write the deterministic dependency-free project used by both A/B arms."""
    (project_dir / "src").mkdir(parents=True, exist_ok=True)
    package = {
        "name": "signal-sweep",
        "version": "1.0.0",
        "private": True,
        "type": "module",
        "scripts": {
            "test": "node --test",
            "check": "node --check src/main.js",
            "serve": "python3 -m http.server 4173",
        },
    }
    starter_files = {
        project_dir / "package.json": json.dumps(package, indent=2) + "\n",
        project_dir / "index.html": STARTER_INDEX,
        project_dir / "src/main.js": STARTER_MAIN,
    }
    for path, content in starter_files.items():
        if not path.exists():
            path.write_text(content, encoding="utf-8")


def write_artifacts(
    output_dir: Path,
    *,
    baseline_prompts: list[str],
    cached_prompts: list[str],
    cached_prompt_names: list[str] | None = None,
) -> None:
    write_starter_project(output_dir / "baseline-project")
    write_starter_project(output_dir / "cached-harness-project")
    baseline_dir = output_dir / "baseline"
    cached_dir = output_dir / "cached_harness"
    baseline_dir.mkdir(parents=True, exist_ok=True)
    cached_dir.mkdir(parents=True, exist_ok=True)
    for prompt_path in (*baseline_dir.glob("*.prompt"), *cached_dir.glob("*.prompt")):
        prompt_path.unlink()
    (output_dir / "signal-sweep-game-brief.md").write_text(
        GAME_DEV_BRIEF, encoding="utf-8"
    )
    for index, prompt in enumerate(baseline_prompts):
        (baseline_dir / f"worker-{index + 1:02d}.prompt").write_text(
            prompt, encoding="utf-8"
        )
    cached_names = cached_prompt_names or [
        f"worker-{index + 1:02d}" for index in range(len(cached_prompts))
    ]
    if len(cached_names) != len(cached_prompts):
        raise ValueError("cached prompt names must match cached prompts")
    if any(
        not name or Path(name).name != name or name.endswith(".prompt")
        for name in cached_names
    ):
        raise ValueError("cached prompt names must be plain stems")
    for name, prompt in zip(cached_names, cached_prompts):
        (cached_dir / f"{name}.prompt").write_text(
            prompt, encoding="utf-8"
        )
    (output_dir / "observations-template.jsonl").write_text(
        "\n".join(
            [
                '{"mode":"baseline","worker":"worker-01","event":"spawned","usage_observed":false,"elapsed_ms":0,"note":"attach normalized telemetry only when observed"}',
                '{"mode":"cached_harness","worker":"worker-01","event":"spawned","usage_observed":false,"elapsed_ms":0,"note":"attach normalized telemetry only when observed"}',
            ]
        )
        + "\n",
        encoding="utf-8",
    )


def format_markdown(report: dict[str, Any]) -> str:
    break_even = report["savings"]["break_even_dispatches"]
    break_even_text = "unreachable" if break_even is None else str(break_even)
    baseline_status = report["status_observations"]["baseline"]
    cached_status = report["status_observations"]["cached_harness"]
    def shown(value: Any) -> str:
        return "unknown" if value is None else str(value)

    def shown_pct(value: Any) -> str:
        return "unknown" if value is None else f"{value}%"

    return f"""# Game Dev A/B Benchmark

Workload: `{report['workload']['name']}` with {report['workload']['workers']} assignments.
Estimator: `{report['estimator']}`. Offline estimates are not provider billing
telemetry.

| Metric | Baseline embedded handoff | Cached harness handoff |
|---|---:|---:|
| Prompt count | {report['runs']['baseline']['prompt_count']} | {report['runs']['cached_harness']['prompt_count']} |
| Estimated tokens total | {report['runs']['baseline']['estimated_tokens_total']} | {report['runs']['cached_harness']['estimated_tokens_total']} |
| Average tokens per prompt | {report['runs']['baseline']['avg_tokens_per_prompt']} | {report['runs']['cached_harness']['avg_tokens_per_prompt']} |
| Cache-adjusted estimated tokens | {report['runs']['baseline']['cache_adjusted_estimated_tokens']} | {report['runs']['cached_harness']['cache_adjusted_estimated_tokens']} |
| Stable prefix tokens counted once | n/a | {report['runs']['cached_harness']['stable_prefix_estimated_tokens_once']} |
| Dynamic tail tokens total | n/a | {report['runs']['cached_harness']['dynamic_tail_estimated_tokens_total']} |
| Stable prefix ratio | n/a | {report['runs']['cached_harness']['stable_prefix_ratio_pct']}% |

Raw estimated savings: `{report['savings']['raw_pct']}%`

Cache-adjusted estimated savings: `{report['savings']['cache_adjusted_pct']}%`

Break-even dispatches: `{break_even_text}`

Equal-quality comparable-sample effective Token savings:
`{shown_pct(report['savings']['observed_runtime_comparable_sample']['total_effective_tokens_pct'])}`

Retry-inclusive operational effective Token savings:
`{shown_pct(report['savings']['observed_runtime']['total_effective_tokens_pct'])}`

## Runtime Status Observations

| Mode | Final status | Events | Execution units closed | Provider input | Noncached input | Cached input | Visible output | Reasoning | Comparable total | Retry total | Operational total |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| baseline | {baseline_status['final_status']} | {baseline_status['event_count']} | {baseline_status['execution_units_closed']} | {shown(baseline_status['provider_input_tokens_total'])} | {shown(baseline_status['input_tokens_total'])} | {shown(baseline_status['cache_read_tokens_total'])} | {shown(baseline_status['output_tokens_total'])} | {shown(baseline_status['reasoning_tokens_total'])} | {shown(baseline_status['comparable_total_effective_tokens'])} | {shown(baseline_status['retry_total_effective_tokens'])} | {shown(baseline_status['total_effective_tokens'])} |
| cached_harness | {cached_status['final_status']} | {cached_status['event_count']} | {cached_status['execution_units_closed']} | {shown(cached_status['provider_input_tokens_total'])} | {shown(cached_status['input_tokens_total'])} | {shown(cached_status['cache_read_tokens_total'])} | {shown(cached_status['output_tokens_total'])} | {shown(cached_status['reasoning_tokens_total'])} | {shown(cached_status['comparable_total_effective_tokens'])} | {shown(cached_status['retry_total_effective_tokens'])} | {shown(cached_status['total_effective_tokens'])} |

Required runtime events: `{", ".join(report['status_protocol']['required_runtime_events'])}`.

Quality gates:
{chr(10).join(f"- `{gate['name']}`: {gate['purpose']}" for gate in report['quality_gates'])}
"""


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--harnessctl",
        type=Path,
        default=default_harnessctl_path(),
        help="Path to the built harnessctl binary.",
    )
    parser.add_argument("--workers", type=int, default=DEFAULT_WORKERS)
    parser.add_argument(
        "--harness-topology",
        choices=HARNESS_TOPOLOGIES,
        default="per-slice",
        help="Generate per-slice Harness prompts or one bounded-batch prompt.",
    )
    parser.add_argument(
        "--format",
        choices=("markdown", "json"),
        default="markdown",
        help="Output format.",
    )
    parser.add_argument("--output", type=Path, help="Optional report output path.")
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="Optional directory for generated A/B prompts and observation template.",
    )
    parser.add_argument(
        "--observations",
        type=Path,
        help="Optional JSONL file with real runtime status/token observations.",
    )
    parser.add_argument(
        "--min-raw-savings-pct",
        type=float,
        default=None,
        help="Optional raw prompt-size gate. Omitted by default.",
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
    if args.workers < 2:
        raise SystemExit("--workers must be at least 2")
    if not args.harnessctl.is_file():
        raise SystemExit(
            f"missing harnessctl binary at {args.harnessctl}; run scripts/build-harnessctl.sh"
        )

    observations = load_observations(args.observations) if args.observations else []
    context_manager: Any
    if args.output_dir:
        args.output_dir.mkdir(parents=True, exist_ok=True)
        context_manager = _StaticWorkDir(args.output_dir)
    else:
        context_manager = tempfile.TemporaryDirectory(prefix="game-dev-ab-")

    with context_manager as tmp:
        work_dir = Path(tmp)
        baseline_prompts = build_baseline_prompts(work_dir, args.workers)
        if args.harness_topology == "bounded-batch":
            cached_prompts = [
                render_cached_batch_prompt(args.harnessctl, work_dir, args.workers)
            ]
            cached_prompt_names = ["batch-01"]
            runtime_topology = corrected_runtime_topology(args.workers)
        else:
            cached_prompts = render_cached_prompts(
                args.harnessctl, work_dir, args.workers
            )
            cached_prompt_names = None
            runtime_topology = per_slice_runtime_topology(args.workers)
        if args.output_dir:
            write_artifacts(
                args.output_dir,
                baseline_prompts=baseline_prompts,
                cached_prompts=cached_prompts,
                cached_prompt_names=cached_prompt_names,
            )
        report = build_game_dev_report(
            harness_prompts=cached_prompts,
            baseline_prompts=baseline_prompts,
            workers=args.workers,
            observations=observations,
            runtime_topology=runtime_topology,
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
    print("OK: game-dev A/B benchmark thresholds passed")
    return 0


class _StaticWorkDir:
    def __init__(self, path: Path) -> None:
        self.path = path

    def __enter__(self) -> Path:
        return self.path

    def __exit__(self, exc_type: object, exc: object, tb: object) -> bool:
        return False


if __name__ == "__main__":
    raise SystemExit(main())
