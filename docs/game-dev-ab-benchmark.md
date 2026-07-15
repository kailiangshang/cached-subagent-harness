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
| Estimated tokens total | 3735 | 2248 |
| Average tokens per prompt | 933.75 | 562.0 |
| Cache-adjusted estimated tokens | 3735 | 865 |
| Stable prefix tokens counted once | n/a | 461 |
| Dynamic tail tokens total | n/a | 404 |
| Stable prefix ratio | n/a | 82.04% |

Raw estimated savings: `39.81%`

Cache-adjusted estimated savings: `76.84%`

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
- Harness follows the current routing decision. Four known compatible ready
  slices must produce one bounded batch before any follow-up is considered.
- Use a follow-up only for later compatible work, after normalized usage is
  recorded, and only while both the accepted-follow-up cap and total-effective
  Token cap remain.
- Nested delegation is forbidden in both arms.
- Capture provider telemetry exactly as exposed; absent fields remain unknown.

The product Dashboard is not an A/B surface. Populate it only from the Harness
run database. Keep Baseline results, comparison tables, and experiment controls
in a separate sanitized benchmark report.

## Runtime Status Observations

During a real run, append lifecycle events and attach Token fields only to an
event that actually carries one complete usage observation:

```json
{"mode":"baseline","worker":"worker-01","event":"spawned","elapsed_ms":0}
{"mode":"baseline","worker":"worker-01","event":"running","elapsed_ms":30000}
{"mode":"baseline","worker":"worker-01","event":"reported","elapsed_ms":180000}
{"mode":"baseline","worker":"worker-01","event":"closed","usage_observed":true,"input_tokens":200,"cache_read_tokens":800,"output_tokens":100,"reasoning_tokens":20,"cache_write_tokens":0,"provider_input_tokens":1000,"provider_output_tokens":120,"elapsed_ms":181000}
{"mode":"baseline","worker":"worker-04","event":"quality_passed","note":"all equal-quality gates passed"}
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
noncached input, cached input, visible output, reasoning, provider totals,
retries, and total effective Tokens. A missing category or worker observation
remains unknown. Runtime savings are calculated only when both arms are closed,
have exact telemetry, and explicitly emit `quality_passed`.

For Codex `turn.completed.usage`, normalize without overlap:

```text
input       = input_tokens - cached_input_tokens
cache_read  = cached_input_tokens
output      = output_tokens - reasoning_output_tokens
reasoning   = reasoning_output_tokens
cache_write = 0
```

`cache_write=0` means this Codex stream exposes no additional category; it does
not infer provider-internal cache behavior. A rejected attempt is emitted as a
`retry` event with its usage and remains in total cost.

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

The 2026-07-15 real run is recorded in
`docs/benchmarks/2026-07-15-signal-sweep-real-ab.md`. It rejected the former
one-Session/three-follow-up strategy at 5.90× Baseline total effective Tokens
and drove the current batch-first, budget-bounded policy.
