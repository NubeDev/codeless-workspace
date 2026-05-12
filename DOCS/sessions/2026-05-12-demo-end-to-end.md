# Build status — full working browser demo against codeless-server

> ⛔ **AGENT REMINDER — READ BEFORE TOUCHING THIS FILE**
>
> 1. You are running JOB-LOOP. Spec: `DOCS/JOB-LOOP.md`. Project scope:
>    `DOCS/SCOPE.md`. Code-style rules: `CLAUDE.md` (repo root).
> 2. **One logical batch per tick.** Read each stage's `[S|M|L]` tag and
>    batch per JOB-LOOP.md "Hard rules" #3.
> 3. **You MUST schedule the next tick before exiting** — call
>    `CronCreate` with `recurring: false` for a single fire ~1 min from
>    now. If all stages are `[x]`, report `DONE`. If you cannot schedule,
>    do NOT exit silently — follow JOB-LOOP.md "If you cannot schedule".
> 4. Update this file in the **same commit** as the code change.
> 5. ⛔ **COMMIT _AND_ PUSH BEFORE THE TICK ENDS.** Never `--force`,
>    never `--no-verify`.
> 6. ⛔ Comments: explain **why**, not what. No emojis, no
>    task-status comments, no decorative banners.
> 7. ⛔ Cross-platform reach is enforceable. UI imports `RpcClient`
>    only; process-spawn lives only in `codeless-adapters-host`.

File: DOCS/sessions/2026-05-12-demo-end-to-end.md
Goal: A user with a fresh checkout can run two commands
      (`codeless serve --fs-root <repo>` + `pnpm dev`) and reach a
      browser UI that lists repos, browses files from the real
      filesystem, opens a file in the editor, submits a mock job,
      and watches it complete via SSE — all against the real
      `codeless-server`, no mocks.
Started: 2026-05-12
Last tick: 2026-05-12 17:38
Current stage: 3 / 8

Repo:        codeless
Branch:      master
Memory policy: each tick fresh; status file is source of truth
Scheduler:   CronCreate one-shot, ~1 min between ticks
Max ticks:   30

## Stages
Format: `[ ] N. [S|M|L] title` — complexity tag is mandatory.

- [x] 1. [S] `fs_cwd` lives in `codeless-rpc::methods::FsCwdResult`,
       trait method `fs_cwd(&self) -> RpcResult<FsCwdResult>`. Runtime
       reads `HostFs::root()`; without `with_fs` it returns the same
       `Internal("fs.* not available")` as the other fs methods.
       Server exposes `/rpc/fs_cwd`; http client wraps it. Both
       snapshots regenerated.

- [x] 2. [S] UI fallback to `fs_cwd` was already wired in `App.tsx`
       from a prior pass — `usePaths().homeDir()` first; if null,
       `rpc.call("fs_cwd", {})`. Stage 1 made the Rust side actually
       answer the call. UI tsc clean. Mock client already implements
       `fs_cwd`. Effective result: opening the browser against a
       real server now shows the workspace contents instead of "no
       current directory".

- [ ] 3. [S] CLI: `codeless demo bootstrap` verb — adds one repo
       row (using the fs root as `local_path`) and submits one mock
       job, idempotent (skips if a repo named "demo" already exists).
       So a fresh database does not greet the user with empty
       repos + empty jobs.

- [ ] 4. [S] UI: empty-state CTA in JobsDashboard — when there are
       zero repos, render a short instruction block instead of an
       empty table ("Run `codeless demo bootstrap` to seed a demo
       repo + mock job, or use Add Repo above"). Keeps the demo
       discoverable for users who skip the README.

- [ ] 5. [M] Make the mock runner actually run end-to-end through
       the server's background driver. The bits exist
       (`spawn_job_driver_loop` is wired in `codeless serve`), but
       the demo bootstrap job needs to actually transition
       Queued -> Running -> Completed when the driver picks it up.
       This is a runtime correctness check + targeted integration
       test if needed; no new public API unless a gap is found.

- [ ] 6. [S] DEMO-UI.md walkthrough at the workspace root: prereqs,
       one-block server-start command, one-block UI-start command,
       what the user should see at each step. Replaces the older
       references in `codeless/README.md` with a single
       authoritative quickstart that's tested against the real
       end-to-end flow.

- [ ] 7. [S] Smoke-test script: a tiny shell script (or a verify
       task in mani.yaml) that boots `codeless serve` on an
       ephemeral port, runs `codeless demo bootstrap`, hits
       `/rpc/list_repos`, `/rpc/list_jobs`, `/rpc/fs_read_dir`, and
       asserts the expected counts. Catches regressions in the demo
       path without depending on a browser.

- [ ] 8. [S] Cleanup pass: any compile/clippy warnings introduced
       during the loop, drift in `wire.ts.snap` not yet committed,
       and a final read of `DEMO-UI.md` against an actual demo run.

## Notes
- The `fs_cwd` method also unlocks the explorer's path bar showing the
  workspace root instead of "/" — small UX win that costs nothing now
  that the method is there.
- Stages 11+12 of the prior session already left the editor + explorer
  on `useRpc()`; this loop only adds the missing demo connective
  tissue.
- The followup-fs methods (`fs_create_*`, `fs_move`, `fs_delete`,
  `fs_search`, `fs_glob`) deliberately stay out of scope — the demo
  goal is read-only browsing + edit + save, plus the jobs surface.

## Blockers
(none)

## Tick log
- Tick 2 (2026-05-12 17:38): stage 2. No code change needed — the
  UI fallback already existed. Stage 1's Rust impl makes it actually
  resolve. The explorer now shows the workspace root on first paint
  against a real server.
- Tick 1 (2026-05-12 17:36): stage 1. `fs_cwd` end-to-end (types,
  trait, runtime, server, client, snapshots).
