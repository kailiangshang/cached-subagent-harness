# Game Dev A/B Benchmark

This benchmark answers a stronger question than the offline token fixture:

Can two equivalent small-game development runs show lower prompt overhead,
auditable worker status, and the same quality gates when one run uses cached
subagent harness prompts?

It still does not call an LLM API by itself. The script generates equivalent
dispatch artifacts and can ingest real runtime observations after you run the
agents in your CLI.

## Why This Exists

The original token-effectiveness task is a CI regression guard. It proves that
the harness keeps reusable instructions in a stable prefix and task-specific
values in a dynamic tail.

That is useful, but it is not a full product claim. In the current refactor
fixture, raw prompt size is larger:

- baseline embedded handoff: `1784` estimated tokens;
- cached harness handoff: `2164` estimated tokens;
- cache-adjusted harness handoff: `856` estimated tokens.

That result means the harness is not an unconditional raw-token reducer. It
needs repeated dispatches and stable-prefix cache hits. The game-dev benchmark
adds a larger, more realistic development brief and a status-observation
protocol so the comparison can include actual worker lifecycle data.

## Workload

The fixed workload is a small browser arcade game named `Signal Sweep`:

- deterministic 12x12 game-state engine;
- responsive rendering and keyboard/touch controls;
- high-score and session JSON export;
- tests plus browser smoke evidence.

The same four worker slices are generated in both modes:

1. engine;
2. rendering and controls;
3. session records;
4. verification and integration.

## Run Offline Estimate

```bash
scripts/build-harnessctl.sh
python3 scripts/game_dev_ab_benchmark.py --format markdown
```

Latest local estimate:

| Metric | Baseline embedded handoff | Cached harness handoff |
|---|---:|---:|
| Prompt count | 4 | 4 |
| Estimated tokens total | 3727 | 2144 |
| Average tokens per prompt | 931.75 | 536.0 |
| Cache-adjusted estimated tokens | 3727 | 836 |
| Stable prefix tokens counted once | n/a | 436 |
| Dynamic tail tokens total | n/a | 400 |
| Stable prefix ratio | n/a | 81.35% |

Raw estimated savings: `42.47%`

Cache-adjusted estimated savings: `77.57%`

Break-even dispatches: `1`

## Generate Real-Run Artifacts

```bash
python3 scripts/game_dev_ab_benchmark.py \
  --output-dir /tmp/game-dev-ab \
  --output /tmp/game-dev-ab/report.json \
  --format json
```

This writes:

- `/tmp/game-dev-ab/baseline-project/`;
- `/tmp/game-dev-ab/cached-harness-project/`;
- `/tmp/game-dev-ab/baseline/worker-01.prompt` through `worker-04.prompt`;
- `/tmp/game-dev-ab/cached_harness/worker-01.prompt` through `worker-04.prompt`;
- `/tmp/game-dev-ab/signal-sweep-game-brief.md`;
- `/tmp/game-dev-ab/observations-template.jsonl`.

The two project directories start with byte-identical, dependency-free
`package.json`, `index.html`, and `src/main.js` files. The starter fixes the
cross-module interfaces while leaving the game slices for the workers. Reusing
the command does not overwrite developed project files, so telemetry reports
can be regenerated safely after a run.

## Real A/B Protocol

Use the generated prompts in two isolated project directories. Before either
run, initialize each project as its own Git repository, create the same starter
commit with the same author metadata, and require equal starter tree hashes.

- Use the same CLI, model, reasoning profile, sandbox, and quality gates.
- Run writes serially in both arms; never overlap workers that can touch the
  same project.
- Baseline uses four fresh sessions, one per worker prompt.
- Harness uses one session plus three accepted follow-ups in worker order.
- Nested delegation is forbidden in both arms.
- Capture provider telemetry exactly as exposed; absent fields remain unknown.

The product Dashboard is not an A/B surface. Populate it only from the Harness
run database. Keep Baseline results, comparison tables, and experiment controls
in a separate sanitized benchmark report.

## Runtime Status Observations

During a real run, append JSONL events with this shape:

```json
{"mode":"baseline","worker":"worker-01","event":"spawned","input_tokens":1200,"output_tokens":0,"elapsed_ms":0}
{"mode":"baseline","worker":"worker-01","event":"running","elapsed_ms":30000}
{"mode":"baseline","worker":"worker-01","event":"reported","input_tokens":100,"output_tokens":1600,"elapsed_ms":180000}
{"mode":"baseline","worker":"worker-01","event":"closed","elapsed_ms":181000}
```

Supported `mode` values are `baseline` and `cached_harness`.

Required lifecycle events are:

- `spawned`;
- `running`;
- `reported`;
- `closed`.

Then rerun:

```bash
python3 scripts/game_dev_ab_benchmark.py \
  --observations /tmp/game-dev-ab/observations.jsonl \
  --format markdown
```

The report aggregates final status, event counts, workers seen, workers closed,
input tokens, output tokens, and observed runtime savings. If observations are
missing, the report says `not-observed` instead of pretending a real run
happened.

## Quality Gates

Both modes must meet the same gates:

- `engine-tests`: game rules, scoring, collision, restart, and timer behavior;
- `build-or-static-smoke`: the game can be served without missing assets;
- `desktop-mobile-screenshot`: game is visible and framed on desktop/mobile;
- `interaction-smoke`: start, move, pause, game-over, and restart work.

## Interpretation

Use three separate claims:

- Raw prompt estimate: what is sent before provider caching.
- Cache-adjusted estimate: stable harness prefix counted once, dynamic tails
  counted per dispatch.
- Runtime observation: actual status and token telemetry from a real agent run.

Only the third claim can prove real end-to-end savings for a specific CLI,
model, cache policy, and task. The offline estimates are regression tests and
planning signals, not billing guarantees.
