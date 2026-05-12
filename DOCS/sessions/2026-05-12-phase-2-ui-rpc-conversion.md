# Build status — Phase 2 UI RPC conversion (fs.* / secrets.* surface)

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
> 7. ⛔ R2 transport boundary is enforceable. No new `@tauri-apps/*`
>    imports outside `src/shells/<shell>/`. Documented exceptions:
>    `src/lib/rpc/tauri-ipc-client.ts`, `src/settings/main.tsx`,
>    and the two PTY files (out of scope for this loop).
> 8. ⛔ Do NOT edit `codeless/crates/*`. UI-only loop. Halt if a
>    stage seems to need a Rust change.

File: DOCS/sessions/2026-05-12-phase-2-ui-rpc-conversion.md
Goal: Drive `@tauri-apps/*` import count from 13 → ≤3 (the two
      documented exceptions plus the two PTY files) by extending
      the RPC surface with `fs.*` / `secrets.*` method types,
      implementing them in `MockRpcClient`, then converting the
      10 blocked UI files to call `useRpc()`. Verified end-to-end
      against `MockRpcClient` at `/?mock=1`.
Started: 2026-05-12
Last tick: 2026-05-12 (stage 4a — MockRpcClient FS + secrets impl)
Current stage: 5 ← next (stage 4b vitest deferred)

Repo:        codeless
Branch:      feat/phase-2a-persistence  (Phase 2 UI work stacks on
             the same branch as Phase 2a/2b/2c per kickoff
             override "KEEP SAME BRANCH")
Memory policy: compact every 3 stages
Scheduler:   CronCreate one-shot, ~1 min between ticks
Max ticks:   30

Scope guardrails:
  - Touches only `codeless/ui/codeless-ui/`. Zero edits in
    `codeless/crates/*`.
  - `fs.*` / `secrets.*` shapes are hand-mirrored from the
    forthcoming Rust additions; stage 15 replaces with specta
    snapshots.
  - Do NOT touch `pty-bridge.ts`, `useTerminalSession.ts`, or
    `settings/main.tsx`.

## Stages
Format: `[ ] N. [S|M|L] title` — complexity tag mandatory.

- [x] 1.  [S] `src/lib/rpc/wire.ts` — FS wire types (`FsEntry`,
          `FsReadResult` union, `FsGrepHit`, `FsGlobHit`).
- [x] 2.  [S] `src/lib/rpc/methods.ts` — `fs_*` method types
          (read_file, write_file, create_file, create_dir,
          read_dir, search, glob, move, delete, cwd).
- [x] 3.  [S] `methods.ts` — `secrets_{set,get,list,rm}` types.
- [x] 4a. [M] `MockRpcClient` — in-memory FS rooted at
          `/home/user/mock-workspace` (seed fixture: README.md,
          src/index.ts, docs/notes.md), in-memory secrets map,
          ~80ms latency, `not_found` / `conflict` /
          `invalid_argument` error kinds. (Note: wire error type
          has no `permission_denied` variant — using
          `invalid_argument` for type-mismatch cases; no
          permission-denied path in mock since there is no real
          permission model.)
- [ ] 4b. [S] Vitest bootstrap + happy-path spec for FS read/
          write/list/delete + secrets set/get/list/rm. Deferred:
          UI package has no vitest dep yet; bootstrap is its own
          stage. Stages 5–12 verify against `/?mock=1` instead.
- [ ] 5.  [S] Convert `modules/editor/lib/useDocument.ts`
          (fs_read_file, fs_write_file). Verify via `/?mock=1`.
- [ ] 6.  [S] Convert `modules/editor/NewEditorDialog.tsx`
          (fs_create_file).
- [ ] 7.  [S] Convert `modules/explorer/lib/useFileTree.ts`
          (fs_read_dir, fs_create_dir).
- [ ] 8.  [S] Convert `modules/explorer/lib/contextActions.ts`
          (fs_move, fs_delete).
- [ ] 9.  [S] Convert `modules/explorer/ExplorerSearch.tsx`
          (fs_search/fs_grep, fs_glob).
- [ ] 10. [S] Convert `modules/statusbar/CwdBreadcrumb.tsx`
          (fs_cwd).
- [ ] 11. [S] Convert `modules/ai/lib/keyring.ts` (secrets_*).
- [ ] 12. [S] Convert `modules/ai/lib/composer.tsx`
          `attachFileByPath` (fs_read_file).
- [ ] 13. [M] Convert `modules/ai/lib/native.ts` — move RPC-able
          surface; halt if any consumer needs pty/shell session.
- [ ] 14. [S] Audit pass — grep `from "@tauri-apps` outside
          `src/shells/<shell>/`; update `DOCS/UI-PORT-AUDIT.md`.
- [ ] 15. [S] Specta codegen wiring — replace hand-mirrored
          `wire.ts` (and `methods.ts` if snapshot exists) with
          codegen output; CI diff script.

## Notes
- Branch override: kickoff specified `feat/phase-2-ui-rpc-conversion`
  but the operator override `KEEP SAME BRANCH` keeps work on
  `feat/phase-2a-persistence` (stacks on Phase 2a/2b/2c).
- Pre-flight commit: the shell-injection capability adapters
  (10 `src/lib/shell/*` + 10 `src/shells/desktop/*` files) were
  uncommitted at tick 0; landed as commit `phase 2: shell-injection
  capability adapters (browser/desktop split)`.
