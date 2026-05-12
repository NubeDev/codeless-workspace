# Build status — Phase 2 UI Stage 15 (specta codegen) + fs.* vertical slice

> ⛔ **AGENT REMINDER — READ BEFORE TOUCHING THIS FILE**
>
> 1. You are running JOB-LOOP. Spec: `DOCS/JOB-LOOP.md`. Project scope:
>    `DOCS/SCOPE.md`. Code-style rules: `CLAUDE.md` (repo root).
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
>    optional and not "later". A tick that ends with unpushed commits
>    means the next tick (or the next agent, after `/clear` or a fresh
>    session) sees stale remote state and can clobber or duplicate work.
>    `./bin/mani --config mani.yaml run commit --projects codeless` then `mani run push --projects
>    codeless` — both, every tick, no exceptions. If push fails, mark
>    the stage `[!]` and halt. Never `--force`, never `--no-verify`.
> 6. ⛔ **CODE COMMENTS ARE LOAD-BEARING — WRITE THEM CAREFULLY.**
>    Comments are how the *next* AI agent (and the next human) understands
>    intent. Rules:
>    - Explain **why**, not what. The code already says what.
>    - **No emojis.** Anywhere. Ever.
>    - **No task-status comments.** Never reference stages, ticks,
>      milestones, "added in stage 3", "TODO from M5", "fixed for ticket
>      X". Comments describe the code as it stands, not the task that
>      produced it.
>    - **Long-term framing.** Write for someone reading this in 6 months
>      with zero context — invariants, constraints, why this approach
>      over the obvious one.
>    - **Normal length.** A short line where one helps. A short paragraph
>      where the *why* is genuinely subtle. No multi-paragraph essays,
>      no decorative banners, no ASCII art.
> 7. ⛔ **CROSS-PLATFORM REACH IS ENFORCEABLE.** Stages that touch Rust
>    crates respect the iOS-safe / Android-safe columns in
>    `DOCS/SCOPE.md` "Crate layout". Stages that touch UI modules import
>    only `RpcClient` — never `@tauri-apps/api/core` directly. Trip
>    either rule → mark stage `[!]` and halt.

File: DOCS/sessions/2026-05-12-phase-2-stage-15-and-fs-slice.md
Goal: Land the deferred Phase 2 UI Stage 15 (specta covers RPC method
      args/results, `wire.ts` + `methods.ts` switch to codegen output,
      CI snapshot check) then ship the `fs.*` RPC vertical slice so
      the Terax file explorer and editor talk to a real `codeless-server`.
Started: 2026-05-12
Last tick: 2026-05-12 17:22
Current stage: 8 / 12

Repo:        codeless
Branch:      master
Memory policy: each tick is a fresh session — status file is the source of truth
Scheduler:   CronCreate one-shot, ~1 min between ticks
Max ticks:   30

## Stages
Format: `[ ] N. [S|M|L] title` — complexity tag is mandatory.
`L` stages must be split into S/M sub-stages before being worked.

Phase A — Specta codegen covers RPC methods (replaces hand-mirrored TS):

- [x] 1. [M] Specta-register all `codeless-rpc::methods` arg/result
       structs (+ subscribe `EventFilter` / `Since`); extend
       `wire.ts.snap` to include them. Add `specta` dependency to
       `codeless-rpc` with the same pinned versions as
       `codeless-types`; keep the crate I/O-free so mobile-reach is
       preserved.
- [x] 2. [S] Add a codegen binary that writes the combined TypeScript
       to `ui/codeless-ui/src/lib/rpc/generated/wire.ts`. Implemented
       as `cargo run -p codeless-rpc --example wire_ts` — example
       targets resolve dev-dependencies, so `specta-typescript` does
       not leak into mobile-reach builds.
- [x] 3. [M] Replaced hand-mirrored core types in
       `ui/codeless-ui/src/lib/rpc/wire.ts` with `export * from
       "./generated/wire"`. Fs/shell forward-declared types stay in
       `wire.ts` until their Rust counterparts land (stages 5–7).
       `methods.ts` is intentionally untouched: the hand-mirrored
       `ListReviewsArgs` disagrees with the Rust shape (UI has
       `job_id`/`pending_only`, Rust has `status`), so swapping it
       cascades into call-site changes that don't fit a single tick.
       Tracked as stage 4b below.
- [x] 4. [S] Drift check landed as a `wire-ts-check` mani task in
       `mani.yaml`: regenerates the TS, then `git diff --exit-code`
       against the committed file. GitHub Actions wiring deferred
       until a CI workflow exists in either repo.
- [x] 4b. [M] Reconciled `ListReviewsArgs`. Added `job_id` to the Rust
       shape (joins through `stages.job_id` in `store.list_reviews`),
       dropped `pending_only` (callers use `status: "pending"` now).
       UI side: `methods.ts` re-exports `ListReviewsArgs`/`Result`,
       `Approve`/`Comment`/`StopReviewArgs` from generated; `ReviewPanel`
       and `mock-client` adopt the new filter shape. Other hand-mirrored
       method-arg types (AddRepoArgs, SubmitJobArgs, etc.) already
       coincidentally match codegen — left for a future cleanup.

Phase B — `fs.*` RPC vertical slice (editor + explorer onto real server):

- [x] 5. [S] `codeless-types::fs` added with `FsEntry` + `FsEntryKind`.
       Method-arg types stay in `codeless-rpc::methods` (stage 6) to
       keep the types crate focused on pure-domain shapes; that mirrors
       how `RemoveRepoArgs`/`GetJobArgs` live in `-rpc`, not `-types`.
- [x] 6. [S] `codeless-rpc::methods` and `RpcServer` extended with
       `fs_read_dir`, `fs_read_file`, `fs_write_file`, `fs_stat`. The
       in-process runtime returns `Internal("not implemented")` for
       all four; the HTTP client routes them through `call`. The
       wire result for `fs_read_file` is the minimal `{ content }`
       — binary / over-limit variants land when the editor needs
       them, not before. Snapshots + generated wire.ts regenerated.
- [x] 7. [M] `codeless-adapters-host::fs::HostFs` implemented with
       `tokio::fs`, scoped to a canonicalised root. Resolves paths via
       `Component` walk then post-canonicalize prefix check so
       symlinks pointing outside the root are caught. 9 unit tests
       cover round-trip, sorted listings, traversal/absolute/symlink
       rejection, non-utf8 typed error, missing-path stat returns
       None, bad-root caught at construction.
- [ ] 8. [S] `codeless-runtime`: hold an `Arc<dyn FsAdapter>` (or the
       concrete `HostFs`) alongside the existing adapters; delegate
       the four new `RpcServer` methods.
- [ ] 9. [S] `codeless-server`: HTTP routes for the four methods (POST
       JSON, mirror the existing RPC route style); error mapping
       reuses `RpcError`. Integration test through `axum::Router`.
- [ ] 10. [S] `codeless-client::HttpRpcClient`: add the four method
        callers + tests against `wiremock` (style already established
        in Phase 3b).
- [ ] 11. [M] UI: convert
        `ui/codeless-ui/src/modules/explorer/lib/useFileTree.ts`,
        `lib/contextActions.ts`, and `ExplorerSearch.tsx` to call
        `useRpc()` instead of `@tauri-apps/*`. Mock client gets a
        small in-memory tree for tests/dev. Zero `@tauri-apps/*`
        imports remain in `modules/explorer/`.
- [ ] 12. [M] UI: convert
        `ui/codeless-ui/src/modules/editor/lib/useDocument.ts` and
        `NewEditorDialog.tsx` to call `useRpc()` for `fs.read_file` /
        `fs.write_file`. Zero `@tauri-apps/*` imports remain in
        `modules/editor/`.

Likely batching (planning hint, not a contract):
- Tick 1: stage 1 (M).
- Tick 2: stages 2 + 3 (S + M, both codegen-adjacent — may split).
- Tick 3: stage 4 (S) alone, or fold into the previous tick if size allows.
- Tick 4: stages 5 + 6 (2× S, both wire-type adjacent).
- Tick 5: stage 7 (M).
- Tick 6: stages 8 + 9 (2× S, runtime + server glue).
- Tick 7: stage 10 (S).
- Tick 8: stage 11 (M).
- Tick 9: stage 12 (M).

## Notes
- Working directly on `master` per user instruction (single-dev workflow).
  No feature branch.
- The specta snapshot already exists for `codeless-types` core types in
  `crates/codeless-types/tests/specta_snapshot.rs`; Stage 1 extends it
  rather than replacing it. The pinned versions are `specta =2.0.0-rc.23`
  and `specta-typescript =0.0.10` — match these in `codeless-rpc`.
- The hand-mirrored TS that Stage 3 replaces is 249 + 290 = 539 lines
  across `wire.ts` and `methods.ts`. Most should disappear into a
  generated module re-export; what remains in hand-written form is the
  small amount of UI-only glue (RpcClient interface, error helpers).
- `fs_*` methods deliberately live on the same `RpcServer` trait as the
  job/repo/review methods, per the SCOPE.md "one enumerable wire schema"
  rationale documented on the trait itself. No second trait.
- `fs_*` does not need a worktree — it operates on the configured server
  root. Worktree-scoped fs lands later, when the editor surfaces job
  workspaces (out of scope for this loop).

## Tick log
- Tick 7 (2026-05-12 17:22): stage 7. HostFs + traversal-rejection
  trust gate. Stat returns Option to let callers probe existence
  without catching NotFound.
- Tick 6 (2026-05-12 17:18): stages 5 + 6. fs types in
  `codeless-types::fs`; method-arg wrappers in `codeless-rpc::methods`;
  trait extended with 4 fs methods; both impls stubbed. Wire shape for
  `fs_read_file` deliberately minimal (just `{ content }`) — binary
  and over-limit variants deferred until the editor needs them.
- Tick 5 (2026-05-12 17:16): stages 4 + 4b. `wire-ts-check` mani task
  works locally. ListReviewsArgs reconciled with `job_id` added on the
  Rust side (proper join), UI call sites updated, codegen regenerated.
- Tick 4 (2026-05-12 17:08): stage 4. Added `wire-ts-check` mani task.
  Known mani-wrapper quirk: it does not propagate the inner script's
  non-zero exit code to its own exit, so this task is suitable as a
  developer/loop affordance but a future GHA step should invoke the
  underlying `cargo run -p codeless-rpc --example wire_ts` and
  `git diff --exit-code` directly to get a hard CI failure.
- Tick 3 (2026-05-12 17:01): stage 3. `wire.ts` now re-exports the
  generated module for core types; fs/shell forward declarations kept
  alongside. Discovered a shape mismatch between UI's `ListReviewsArgs`
  (`job_id`, `stage_id`, `pending_only`) and Rust's (`stage_id`,
  `status`); cascades into UI call sites if reconciled in this tick.
  Split it out as stage 4b rather than expand stage 3's diff. UI tsc
  passes; Rust unchanged but clean.
- Tick 2 (2026-05-12 16:57): stage 2. Codegen lives as `cargo run -p
  codeless-rpc --example wire_ts`. Picked example over `[[bin]]` because
  examples resolve dev-dependencies; that keeps `specta-typescript` out
  of the mobile-reach dependency graph entirely, where a `[[bin]]` would
  have either polluted the normal deps or required a separate tool
  crate. Output deterministic (md5 stable across runs), ~10.7 KB,
  302 lines, single file at `ui/codeless-ui/src/lib/rpc/generated/wire.ts`.
- Tick 1 (2026-05-12 17:02): stage 1. Added `specta` to `codeless-rpc`,
  derived `Type` on all method arg/result structs + `EventFilter`. Kept
  the snapshot per-crate (new `codeless-rpc/tests/wire-rpc.ts.snap`)
  rather than cross-crate to avoid a `codeless-types -> codeless-rpc`
  dev-dep cycle. Both snapshots will be concatenated by the stage 2
  codegen binary.

## Blockers
(none)
