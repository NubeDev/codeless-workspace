# Build status — Phase 2c CLI completion (real runners + reviews + notifier)

> ⛔ AGENT REMINDER — READ BEFORE TOUCHING THIS FILE
>
> 1. You are running JOB-LOOP. Spec: `DOCS/JOB-LOOP.md`. Project scope:
>    `DOCS/SCOPE.md`. Code-style rules: workspace `CLAUDE.md` and
>    `codeless/CLAUDE.md`.
> 2. One logical batch per tick. Read each stage's `[S|M|L]` tag and
>    batch per JOB-LOOP.md "Hard rules" #3.
> 3. You MUST schedule the next tick before exiting — `CronCreate`
>    recurring:false ~1 min out. If all stages `[x]`, report DONE.
> 4. Update this file in the same commit as the code change.
> 5. ⛔ COMMIT AND PUSH via mani every stage. Never `--force`, never
>    `--no-verify`. If push fails, mark `[!]` and halt.
> 6. ⛔ Comments explain *why*. No emojis. No task-status comments.
> 7. ⛔ R1 cross-platform reach is enforceable. Process spawn stays in
>    `codeless-adapters-host` (and `ai-runner`). Mobile-safe crates
>    must not pull `tokio::process` or `claude-wrapper` transitively.

File: DOCS/sessions/2026-05-12-phase-2c-cli-completion.md
Goal: Finish the remaining SCOPE.md Phase 2 deliverables — CLI
      wiring of the real runners, YAML job-template loader, review
      approval CLI, `codeless tail`, and the outbound notification
      webhook (Notifier trait + generic backend) — so Phase 2 is
      ready for browser-shell work (Phase 3) without further CLI
      churn.
Started: 2026-05-12
Last tick: 2026-05-12 (stage 1)
Current stage: 2 / 6

Repo:        codeless
Branch:      feat/phase-2a-persistence  (Phase 2c stacks on the same
             branch as Phase 2a + 2b — combined PR cuts at end of
             stage 6)
Memory policy: compact every 3 stages
Scheduler:   CronCreate one-shot, ~1 min between ticks
Max ticks:   30

## Stages
Format: `[ ] N. [S|M|L] title` — complexity tag mandatory.

- [x] 1. [M] CLI runner selection: `codeless run --runner {claude,
         anthropic,mock}` wires the right adapter from
         `codeless-runtime` into `drive_job`. Default stays `mock`
         so existing tests keep passing. Test uses the fake `claude`
         binary on explicit PATH (per SCOPE testing strategy) and
         asserts events stream through to stdout JSON-line output.
- [!] 2. [S] CLI review surface: `codeless review {list,approve,  ← halted
         comment,stop}` against existing review state machine. No
         new RPC methods — just wire the existing ones to clap
         subcommands.
- [ ] 3. [M] YAML job template loader: `codeless job submit
         job.yaml` parses `{repo, runner, prompt, stages, caps}`
         and calls `submit_job`. Test fixture exercises a 2-stage
         job; round-trip the YAML through serde so a syntax error
         surfaces with line/col.
- [ ] 4. [S] CLI tail: `codeless tail <job-id>` subscribes to
         events for the job and streams JSON-line output to stdout
         until terminal status. Reuses the subscriber path that
         `run --once` already exercises.
- [ ] 5. [M] Notifier trait + generic webhook backend. Triggers on
         `JobFailed` + `ReviewRequested`. Config lives alongside
         the secrets file (single-tenant). Backend posts JSON to a
         configurable URL with HMAC signing; test against a
         `wiremock` fixture asserting payload shape + signature
         header.
- [ ] 6. [S] Phase 2c wrap-up: CODELESS.md refresh, README
         quickstart updated with the new CLI surfaces, three verify
         gates green, branch ready for combined Phase 2a + 2b + 2c
         PR.

Likely batching:
- Tick 1: stage 1 (M).
- Tick 2: stage 2 (S) — could pair with stage 4 (S) if both fit.
- Tick 3: stage 3 (M).
- Tick 4: stage 4 (S) if not already in tick 2; else stage 5 (M).
- Tick 5: stage 5 (M).
- Tick 6: stage 6 (S) — wrap-up + DONE.

## Notes
- Phase 2a + 2b are committed and pushed on
  `feat/phase-2a-persistence`. Phase 2c stacks on top; the combined
  PR cuts at the end of stage 6.
- The UI (`codeless/ui/codeless-ui/`) is untouched by Phase 2c — UI
  work is a separate workstream tracked in
  `DOCS/UI-PORT-AUDIT.md`. CLI surfaces here become available to
  the UI as RPC methods automatically.
- New CLI subcommands should preserve the existing JSON-line output
  format so `codeless tail <job-id> | jq` continues to work.

## Notes (tick log)
- Stage 1: `RunnerKind` clap ValueEnum on `RunArgs`; `build_runner`
  dispatches to `MockRunner` / `ClaudeRunnerAdapter` /
  `AnthropicRunnerAdapter`. `--api-key` and `--base-url` flags added
  for the anthropic path; key falls back to `ANTHROPIC_API_KEY`.
  Integration test `run_with_claude_runner_streams_ai_events`
  installs the fake `claude` binary used by phase 2b and asserts
  `ai-token` + `ai-message-complete` + `job-completed` reach stdout.

## Blockers

- Stage 2 misspecified (2026-05-12, tick 2). The stage says "wire
  the **existing** review RPC methods to clap subcommands. No new
  RPC methods." But `codeless_rpc::RpcServer` exposes only
  add_repo/remove_repo/list_repos/submit_job/get_job/list_jobs/
  stop_job/subscribe — no list_reviews / approve_review /
  comment_review / stop_review methods exist. The runtime has the
  review state machine and the `reviews` table, but there is no
  RPC surface to drive them. R4 ("new CLI commands go through
  codeless-rpc methods, not directly against the DB") forbids the
  CLI from bypassing the RPC layer.

  Resolution paths for the human:
  (a) Rescope stage 2 to M and add the four review RPC methods +
      the CLI wiring in one stage.
  (b) Split into a new stage 2a (M, add review RPC methods +
      runtime impl + tests) followed by stage 2b (S, clap wiring).
  (c) Skip stage 2 for Phase 2c and defer review-CLI to a later
      phase.

  Loop halted. No next tick scheduled.
