# Build status — Phase 2b real runners + worktree threading

> ⛔ **AGENT REMINDER — READ BEFORE TOUCHING THIS FILE**
>
> 1. You are running JOB-LOOP. Spec: `DOCS/JOB-LOOP.md`. Project scope:
>    `DOCS/SCOPE.md`. Code-style rules: workspace `CLAUDE.md` and
>    `codeless/CLAUDE.md`.
> 2. **One logical batch per tick.** Read each stage's `[S|M|L]` tag and
>    batch per JOB-LOOP.md "Hard rules" #3: up to 4 contiguous S in one
>    area, OR 1 M (+ optional related S), OR the next sub-stage of an L.
>    Verify + commit + push **each stage** via mani before moving to the
>    next stage in the batch.
> 3. **You MUST schedule the next tick before exiting** — call
>    `CronCreate` with `recurring: false` for a single fire ~1 min from
>    now. If all stages are `[x]`, report `DONE` instead. If you cannot
>    schedule, **do NOT exit silently** — tell the user which stage
>    finished, exactly why scheduling failed, and how to re-kick. See
>    JOB-LOOP.md "If you cannot schedule".
> 4. Update this file in the **same commit** as the code change.
> 5. ⛔ **COMMIT _AND_ PUSH BEFORE THE TICK ENDS.** Pushing is not
>    optional and not "later".
>    `./bin/mani --config mani.yaml run commit --projects codeless` then
>    `mani run push --projects codeless` — both, every tick, no
>    exceptions. If push fails, mark the stage `[!]` and halt. Never
>    `--force`, never `--no-verify`.
> 6. ⛔ **CODE COMMENTS ARE LOAD-BEARING — WRITE THEM CAREFULLY.**
>    Explain **why**, not what. No emojis. No task-status comments.
>    Long-term framing. Normal length.
> 7. ⛔ **CROSS-PLATFORM REACH IS ENFORCEABLE.** Process-spawning runners
>    live only in `codeless-adapters-host`. Mobile-safe crates
>    (`codeless-types`, `codeless-rpc`, `codeless-client`) must not
>    pull in `tokio::process` or `claude-wrapper` transitively. Trip →
>    mark stage `[!]` and halt.

File: DOCS/sessions/2026-05-12-phase-2b-runners.md
Goal: Adopt the vendored `ai-runner` crate, run real coding runners
      (`ClaudeRunner` for CLI-wrapped, `AnthropicRunner` for direct
      API) end-to-end through `drive_job`, thread `WorktreeManager`
      so every job runs in its own checkout, and add cost tracking
      with cap-driven cancellation.
Started: 2026-05-12
Last tick: 2026-05-12 (init)
Current stage: 1 / 7

Repo:        codeless
Branch:      feat/phase-2a-persistence  (Phase 2b stacks on the same
             branch — Phase 2a has not yet been PR'd, so the work
             builds on top rather than cutting `feat/phase-2b-runners`)
Memory policy: compact every 3 stages
Scheduler:   CronCreate one-shot, ~1 min between ticks
Max ticks:   30

## Stages
Format: `[ ] N. [S|M|L] title` — complexity tag is mandatory.

- [ ] 1. [M] Adopt the vendored `ai-runner` crate as a workspace member  ← next
         and add `codeless-adapters-host::ai_runner_bridge` translating
         `ai-runner`'s `mpsc::Sender<RunnerEvent>` output into
         `codeless-types::Event` and publishing through `EventBus`.
         Keep our local `Runner` trait + `MockRunner` working alongside
         as the scriptable test path.
- [ ] 2. [M] Worktree-per-job: `drive_job` creates a `git worktree`
         via `WorktreeManager` before invoking the runner, threads
         the worktree path into `RunnerContext`, and removes the
         worktree on terminal status. Test pins lifecycle (existence
         during `running`, cleanup on `completed`/`failed`/`stopped`).
- [ ] 3. [M] `ClaudeRunner` wired end-to-end through the bridge.
         Tests use a fake `claude`-style binary on an explicit `PATH`
         (SCOPE.md "Testing strategy") — never the developer's host
         install. Asserts a stage's events land via the bridge in the
         expected order.
- [ ] 4. [M] `AnthropicRunner` wired end-to-end through the bridge.
         Test uses `wiremock` (or similar) to fake the Anthropic API
         and asserts cost numbers from the response land on
         `ai-message-complete` envelopes.
- [ ] 5. [S] Cost rollup: incoming `ai-message-complete` events
         increment `jobs.cost_cents` and the affected task's
         `cost_cents` row. Test asserts running totals across a
         multi-message session.
- [ ] 6. [M] Cost-cap cancellation: `drive_job` watches the running
         cost against `job.cost_cap_cents`; when the cap is reached,
         it cancels the runner via `tokio_util::sync::CancellationToken`
         and emits `job-stopped { reason: cost-cap }`. Wall-clock cap
         lands here too via the same cancellation channel.
- [ ] 7. [S] Phase 2b wrap-up: CODELESS.md refresh, README quickstart
         showing a non-mock runner invocation, three verify gates
         green, branch ready for PR (Phase 2a + 2b stacked).

Likely batching (planning hint, not a contract):
- Tick 1: stage 1 (M).
- Tick 2: stage 2 (M).
- Tick 3: stage 3 (M).
- Tick 4: stage 4 (M).
- Tick 5: stages 5 + 6 (S + M, both cost-adjacent).
- Tick 6: stage 7 (S) — wrap-up + DONE.

## Notes
- The vendored `ai-runner` crate sits at `ai-runner/` in the workspace
  (one level up from the inner `codeless` repo). Workspace member path
  is `../ai-runner` from the `codeless` Cargo workspace root. Pin via
  `path =` rather than the upstream `rubix-agent` crate so the loop
  never depends on network access to advance.
- `ai-runner` has its own `Runner` trait shaped around an
  `mpsc::Sender<RunnerEvent>`. Our `codeless-runtime::runner::Runner`
  shape is bus-aware. The bridge in `codeless-adapters-host` adapts
  one to the other — converting each `RunnerEvent` to a
  `codeless-types::Event` and forwarding via `EventBus::publish`.
- Process-spawning is the cross-platform tripwire here. The
  `claude-wrapper` and `anthropic-ai-sdk` deps live in `ai-runner`,
  which is host-only. The bridge crate is `codeless-adapters-host`,
  also host-only. Mobile-safe crates (`-types`, `-rpc`, `-client`)
  never see any of this. R1 from workspace `CLAUDE.md` applies — if
  a stage adds `use std::process` or `use tokio::process` anywhere
  but `codeless-adapters-host` or `ai-runner`, mark `[!]` and halt.
- Phase 2a's branch is `feat/phase-2a-persistence` and remains
  unmerged. Phase 2b commits stack on top. The PR for the combined
  branch happens at the end of stage 7.

## Blockers
(none)
