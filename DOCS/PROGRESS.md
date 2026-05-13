# PROGRESS — where codeless is, what's in flight, what's next

> Snapshot taken 2026-05-13. Use this when you sit down at the keyboard
> and need to know "what shipped, what's stuck where, what should I do
> next." It supersedes session notes scattered under
> `DOCS/sessions/` — those are the *how*; this is the *what* and
> *what's next*.

## What's on master, working

The inner `codeless/` repo at HEAD (`eb0293d`) carries a real,
end-to-end MVP:

### Runtime + UI core
- Single-tenant SQLite source of truth (`jobs`, `stages`, `tasks`,
  `events`, `reviews`).
- `RpcServer` trait + `InProcessRpc` implementation.
- Background job-driver loop with concurrency cap, lease reaper, and
  per-runner factory selection.
- Browser UI on Vite (`http://127.0.0.1:1420`) with the full job
  surface: dashboard, JobPage with tabs (Stages, Timeline,
  Files changed, Handover, Spec, Worktree).
- HTTP+SSE transport via `codeless-server`. Loopback unauthenticated;
  non-loopback forces `--require-token`.
- Wire types generated via specta into a TS snapshot
  (`crates/codeless-rpc/tests/wire-rpc.ts.snap`) — drift becomes a
  test failure.

### Runners
- `MockRunner` — scripted, no process spawn. Drives the entire event
  shape without external dependencies.
- `ClaudeRunnerAdapter` — wraps the host `claude` CLI through the
  vendored `ai-runner` crate. Per-job model / permission-mode /
  effort overrides (these landed last 24h — see "Sharp edges" below).
- `AnthropicRunnerAdapter` — direct REST against the Anthropic API.
- `CopilotRunnerAdapter` — GitHub Copilot CLI wrapper.
- `TemplateRunner` — orchestrates multi-stage jobs by spinning up a
  per-stage Claude (or per-stage Mock with my in-flight work, see
  below) and stitching their events into one job.

### Iterate loop (job-as-directory)
- `.codeless/jobs/<name>/` layout with `template.yaml`, `SCOPE.md`,
  `WORKFLOW.md`, plus any other `.md`. Spec pane in the UI lets the
  user edit any of them inline.
- `list_job_files` / `read_job_file` / `write_job_file` /
  `delete_job_file` RPCs.
- Per-stage docs (the loop landed this *after* the original
  JOB-DIR.md design): each stage's `StageSpec` can carry its own
  ordered docs list scoped to that stage; the prompt builder folds
  per-stage docs in after the global SCOPE/WORKFLOW.
- Re-run dialog with optional feedback that lands as a note before
  the fresh job is queued.
- Handover panel with the structured editor + `write_handover` RPC.

### Tools (Phase 2-5, all last 24h)
- `codeless-tools` crate — shared policy helpers, ported from moxxy
  with attribution in `NOTICE`.
- `http.request` tool with full policy enforcement.
- `browser.*` tool surface backed by a Playwright sidecar:
  `session.open/close/list`, `navigate`, `read` (markdown / text),
  `screenshot`, `eval`, `extract`, `wait`, `cookies`,
  `click/type/fill/hover/scroll`, `crawl`.
- `codeless-mcp` — stdio MCP server exposing the tool surface so
  external MCP clients can use codeless's tools.

### Demo path
- `codeless demo bootstrap` seeds a repo + a mock job.
- `codeless demo bootstrap --target-self` registers the inner repo
  as a Claude-runnable target with no auto-queued mock.
- `DOCS/START-SERVER-UI.md` walks the two-terminal-one-browser flow.

## What's in flight on the working tree (not committed)

Two pieces of mine, currently uncommitted, both broken against
current master because the loop landed Job-runner overrides
(`model`, `permission_mode`, `effort`) on `SubmitJobArgs` after I
started.

### Stage rollups (the work the user can see in the screenshot today)

**Goal:** show per-stage `duration` and `cost_cents` on the Stages
tab so a completed job reads as "scaffold main.go — 253ms — $0.02"
instead of just a checkmark.

**Approach:** observer pattern. A new `StageRecorder` tails the
event bus and writes `stages` + `tasks` rows. The runtime serves
rolled-up data through a new `list_stages` RPC.

**Files I touched** (uncommitted):
- `crates/codeless-types/src/event.rs` — `StageStarted` gained
  `ordinal` + `name` fields (`#[serde(default)]` so old envelopes
  still parse).
- `crates/codeless-runtime/src/template_runner.rs` — emit the new
  event fields; opt-in `use_mock_runner` flag so non-claude template
  jobs still drive the recorder.
- `crates/codeless-runtime/src/store.rs` — `update_stage_completed`,
  `list_stages_for_job` (joins tasks for cost rollup),
  `insert_task_minimal`, `add_task_cost`, `mark_task_terminal`.
- `crates/codeless-runtime/src/stage_recorder.rs` — NEW. Subscribes
  to the bus, persists Stage/Task rows from `StageStarted` /
  `StageCompleted` / `TaskStarted` / `AiMessageComplete` /
  `TaskCompleted` events. One unit test passing.
- `crates/codeless-runtime/src/lib.rs` — registers `stage_recorder`,
  re-exports `spawn_stage_recorder`.
- `crates/codeless-runtime/src/rpc.rs` — `list_stages` impl, uses
  the store helper.
- `crates/codeless-rpc/src/methods.rs` — `ListStagesArgs/Result`,
  `StageRollup`.
- `crates/codeless-rpc/src/server.rs` — trait method.
- `crates/codeless-rpc/src/lib.rs` — re-exports.
- `crates/codeless-rpc/tests/specta_snapshot.rs` — registered new
  types. Snapshot regenerated.
- `crates/codeless-server/src/routes.rs` — `/rpc/list_stages` route.
- `crates/codeless-client/src/http_client.rs` — HttpRpcClient impl.
- `crates/codeless-cli/src/serve.rs` — spawn `StageRecorder` at
  boot; mock+template fallback so iterate-loop demos without
  `--enable-claude`.
- `ui/codeless-ui/src/lib/rpc/methods.ts` — TS wire types +
  `RpcMethodMap` entry.
- `ui/codeless-ui/src/lib/rpc/index.ts` — re-exports.
- `ui/codeless-ui/src/modules/jobs/StageTree.tsx` — calls
  `list_stages`, renders duration + cost per stage, total summary
  in the header. Pre-rollup jobs fall back to event-only rendering.

**Smoke-tested live**: a mock-template job produced 3 stage rows
with ~250ms each and 1, 2, 3 cents respectively. Job-level cost
matched the sum. The UI rendered all of it.

### `HostFs` extra roots (the path-escapes-root fix)

**Goal:** the UI Handover/Notes panes read worktree files through
`fs_read_file`. The host adapter previously had one root; worktree
files lived outside it, so the call errored with
`invalid_argument: path escapes root`.

**Approach:** `HostFs` now holds optional `extra_roots`. The trust
gate accepts paths under the primary root *or* any extra root. The
server registers `--worktree-root` (or its default) as an extra
root at boot. Writes still flow through typed RPCs only —
`fs_write_file` against the extra root is not opened up by this.

**Files I touched** (uncommitted):
- `crates/codeless-adapters-host/src/fs.rs` — `extra_roots` field,
  `with_extra_root` builder, `allowed_under_any_root` check.
- `crates/codeless-cli/src/serve.rs` — wires `--worktree-root`
  through `HostFs::with_extra_root`.

**Smoke-tested live**: handover file at `/tmp/codeless-worktrees/
job-<id>/runs/<id>/handover.md` now reads through `fs_read_file`.

## Sharp edges right now

### My branch doesn't build against master

The loop landed `model`/`permission_mode`/`effort` on `SubmitJobArgs`
and a `parse_permission_mode` helper while I was working. My
working tree references `SubmitJobArgs` in places that the new
fields don't reach, and one call site in `serve.rs` calls
`parse_permission_mode` without importing it. **Four call-site
errors total** — all in `codeless-cli`. None in `codeless-runtime`
or `codeless-rpc`, which means my actual feature code is fine; the
mismatch is at integration points.

### Stage recorder vs. existing lease path
`StageRecorder::upsert_task_started` uses `INSERT OR IGNORE` so the
existing lease-driven task path (in `Store::enqueue_task`) wins when
both fire. Template-runner jobs don't go through the lease path, so
in practice the recorder is the sole writer; the OR-IGNORE is the
belt-and-braces. Worth a dedicated test before this lands.

### No claude session ID, no tool-call ribbon, no commit subjects per stage
The wedge ships only duration + cost. The user's original screenshot
asked for *session id, time taken, even more of the steps and
details*. Time is in; cost is in; the rest is staged but not built.
See "Next steps" below.

### `codeless-mcp` upstream API drift
`rmcp::model::Tool` went non-exhaustive in a recent crate release;
`crates/codeless-mcp/src/handler.rs` builds it with a struct
expression and the crate fails to compile. Not my work — this was
broken before I started. The MCP server isn't on the boot path of
the browser demo so the demo still works; it does block
`cargo check --workspace`.

## Merge plan — getting in-flight work onto master

Order matters. Land the smaller, narrower changes first so the
larger ones rebase cleanly.

### Pass 1 — fix the SubmitJobArgs call sites (S, 10 min)

Three CLI subcommands construct `SubmitJobArgs` without the three
new fields. Add `model: None, permission_mode: None, effort: None`
to each site. Same fix for the call in `serve.rs`. Verify with
`cargo check -p codeless-cli`.

Files:
- `crates/codeless-cli/src/demo.rs:125`
- `crates/codeless-cli/src/job.rs:114`
- `crates/codeless-cli/src/run.rs:72`
- `crates/codeless-cli/src/serve.rs:555` — also resolve
  `parse_permission_mode` (import from `codeless_runtime` per the
  new re-export).

After this: `cargo build -p codeless-cli --bin codeless` is green.

### Pass 2 — land HostFs extra roots (S, 20 min)

Smallest of the two feature commits. No wire changes, no migrations.

1. `crates/codeless-adapters-host/src/fs.rs` — the `extra_roots`
   support.
2. `crates/codeless-cli/src/serve.rs` — wire `--worktree-root`.
3. Add a focused unit test in `fs.rs`: register two roots, prove a
   path under the second resolves; prove a path under neither still
   errors with `Escape`.
4. `cargo test -p codeless-adapters-host`.
5. Commit: `host-fs: allow extra readable roots (worktree handover/notes)`.

### Pass 3 — land Stage rollups (M, 1-2h)

Bigger surface. Three sub-commits keeps `git log` readable.

**3a — wire types + RPC plumbing.** Touches `codeless-types/event.rs`
(StageStarted carries name+ordinal), `codeless-rpc/methods.rs`
(StageRollup, ListStagesArgs/Result), `codeless-rpc/server.rs` and
`lib.rs` (trait + re-exports), `codeless-rpc/tests/specta_snapshot.rs`,
`codeless-rpc/tests/wire-rpc.ts.snap`, `codeless-server/routes.rs`
(handler + route), `codeless-client/http_client.rs`. Compile-only
change at runtime — no behaviour yet.
Commit: `stage-rollups: wire types + list_stages RPC plumbing`.

**3b — store helpers + StageRecorder.** Touches `runtime/store.rs`
(four new methods), `runtime/stage_recorder.rs` (new module),
`runtime/lib.rs` (module reg + re-export), `runtime/rpc.rs`
(`list_stages` impl). Add the existing unit test from
`stage_recorder.rs` plus one integration test that submits a
mock-template job, polls `list_stages`, asserts rollup correctness.
Commit: `stage-rollups: StageRecorder + per-stage cost/duration`.

**3c — UI + template_runner.** Touches `runtime/template_runner.rs`
(opt-in mock runner, emit name+ordinal on StageStarted),
`cli/serve.rs` (spawn the recorder, mock+template fallback),
`ui/codeless-ui/src/modules/jobs/StageTree.tsx`,
`ui/codeless-ui/src/lib/rpc/{methods,index}.ts`. Smoke test in the
browser after restart.
Commit: `stage-rollups: UI + mock template fallback`.

### Pass 4 — verify and push

- `cargo test --workspace` green (modulo the pre-existing
  `codeless-mcp` build break — that's its own ticket).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo fmt --check` clean.
- `npx tsc --noEmit` in `ui/codeless-ui` clean.
- Manual smoke: restart server, submit a mock-template job, click
  the Stages tab, confirm durations + costs render. Click Handover
  tab, confirm the panel populates (no `path escapes root`).

### Pass 5 — push

```
git push origin master
```

Phase 2-5 work is already pushed; this just adds the stage-rollup
+ host-fs commits on top.

## Next steps after merge

Roughly priority order. Each is independent — pick whichever fits
the next session.

### Stages tab: the rest of the user's wishlist (M, half a day)

The wedge that just shipped covers *duration* and *cost*. The
screenshot ask was richer:

1. **Claude session ID** per stage. `ClaudeRunnerAdapter` already
   sees the session UUID in the JSON stream; capture it on the
   `Stage` row (`session_id TEXT NULL` migration on `stages`).
   Display + copy button on the expanded stage row.
2. **Per-stage commit subjects.** The runner writes commits inside
   the worktree; `git log <branch>` knows what landed when. Join
   with stage timestamps and surface as a clickable list on the
   stage row (→ Files changed tab pre-filtered).
3. **Tool-call ribbon.** Compact chronological list:
   `Bash(go build) · Read(main.go) · Edit(main.go)`. The events
   are already in the timeline; just roll them up the same way the
   recorder rolls up cost.
4. **Final message excerpt.** Last `AiMessageComplete` per stage,
   truncated to ~5 lines, with "show all" toggle.
5. **Inline expand.** Click a stage row → it expands in place. Today
   the row is a flat line.

Schema impact: one new column (`session_id`). Everything else is UI
rollup over events the recorder already sees.

### Fix `codeless-mcp` build break (S, 30 min)

`rmcp::model::Tool` went non-exhaustive. Use `Tool::builder()` or
field-init shorthand with `..Default::default()`. Currently blocks
`cargo check --workspace` (and therefore CI when it lands).

### Claude commits the handover (S, 30 min)

JOB-MODEL.md says session N must commit `handover.md` + `log.md`
together as `handover N`. Today the runtime writes the files but
doesn't commit. Surface this once it bites (claude jobs that fail
to commit their own handover land at the next-session pickup with
no clean contract). The host-side helper is already there
(`commit_paths` in `adapters-host/git_commit.rs`).

### Live SSE-driven Stage row updates (S-M)

The Stages tab currently re-fetches `list_stages` on every
`stage-completed` envelope. Cheap, but a busy job means many
re-fetches. Could subscribe and apply deltas client-side, with
`list_stages` as the catch-up source on mount only. Polish, not
critical.

### Dogfood the iterate loop against codeless itself (M)

The plumbing is in. What's missing: a real run that uses SCOPE.md +
WORKFLOW.md to drive Claude against the codeless repo, end-to-end,
with the user watching from the browser. The point of every
previous session. The kickoff prompt format is captured in
`DOCS/JOB-LOOP-KICKOFF.template.md`.

### Promote stage rollups from derived → first-class (L, future)

Today `cost_cents` rolls up from `tasks` on every `list_stages`
call. Fine at this scale. If the Stages tab grows (commits, tool
calls, session id, etc.) the cost of each rollup grows with it;
adding the columns to the `stages` row stops the recomputation
without changing the contract. The migration is small. Don't do it
until you've felt the cost.

### Persist Stage rows reliably across recorder restarts (S-M)

`subscribe_since(All, None)` is live-only, so a recorder restart
loses every event during the gap. The events table is the durable
source of truth — a one-shot replay at startup
(`subscribe_since(All, Some(last_seen_cursor))`) fixes this. Needs
a `last_seen_cursor` field on a tiny `recorder_state` table.

## Documentation that's now stale (housekeeping)

- `DOCS/JOB-DIR-KICKOFF.md` references the destroyed session and a
  rebuild plan that's now done. Delete or mark "historical".
- `DOCS/JOB-DIR.md` "What this rebuilds" section at the bottom is
  obsolete — the rebuild happened. Trim that section so the doc
  reads as a design spec, not a status doc.
- `DEMO-UI.md` `ux-12`–`ux-15` entries describe the iterate-loop UI
  that's now on master; the descriptions are accurate but the
  "what landed" framing is wrong (they're from before the rebuild).
  One-line tweak per entry.

## TL;DR for the next session

1. Read this doc.
2. Run **Pass 1** through **Pass 5** to get my uncommitted work onto
   master. ~2-3 hours total, mostly mechanical.
3. After that, the Stages-tab-richer item is the natural next move —
   it's what the user asked for in the screenshot, and the recorder
   is already in place to feed it.
