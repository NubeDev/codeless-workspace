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
Last tick: 2026-05-12 16:57
Current stage: 3 / 12

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
- [ ] 3. [M] Replace hand-mirrored `ui/codeless-ui/src/lib/rpc/wire.ts`   ← next
       and `methods.ts` with re-exports of the generated module. UI
       must still typecheck; `MockRpcClient`, `HttpSseClient`, and
       `TauriIpcClient` keep working unchanged.
- [ ] 4. [S] CI snapshot check: a GitHub Actions step (or `mani` task)
       that runs the wire-ts codegen and `git diff --exit-code` against
       the committed generated file, so drift between Rust types and
       UI types becomes a CI failure.

Phase B — `fs.*` RPC vertical slice (editor + explorer onto real server):

- [ ] 5. [S] `codeless-types`: add `FsEntry`, `FsEntryKind`, plus
       `FsReadDirArgs`, `FsReadDirResult`, `FsReadFileArgs`,
       `FsReadFileResult`, `FsWriteFileArgs`, `FsStatArgs`,
       `FsStatResult`. Pure data, serde + specta. Register in the
       snapshot.
- [ ] 6. [S] `codeless-rpc::methods`: add the matching method-arg
       wrappers; extend `RpcServer` trait with `fs_read_dir`,
       `fs_read_file`, `fs_write_file`, `fs_stat`. Wire-ts snapshot
       updates.
- [ ] 7. [M] `codeless-adapters-host::fs`: a single host implementation
       backed by `tokio::fs`, scoped to a configured root (refuse
       traversal outside the workspace root). Unit tests against a
       `tempfile::tempdir`. R1 reminder: this is the only crate
       allowed to touch the OS filesystem.
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
