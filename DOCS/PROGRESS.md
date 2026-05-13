# PROGRESS — where codeless is, what's in flight, what's next

> Snapshot taken 2026-05-13 (late session). Use this when you sit
> down at the keyboard and need to know "what shipped, what's stuck
> where, what should I do next." Supersedes session notes scattered
> under `DOCS/sessions/`.

## What's on master, working

The inner `codeless/` repo at HEAD (`b30663f` `agent_chat: route
footer AI panel to host CLI runners`) carries a real, end-to-end MVP.

### Runtime + UI core
- Single-tenant SQLite source of truth (`jobs`, `stages`, `tasks`,
  `events`, `reviews`).
- `RpcServer` trait + `InProcessRpc` implementation.
- Background job-driver loop with concurrency cap, lease reaper, and
  per-runner factory selection.
- Browser UI on Vite (`http://127.0.0.1:1420`) with the full job
  surface: dashboard, JobPage with three-section sidebar (SPEC /
  STAGES / RUN), per-stage drilldown, header [edit spec] +
  [re-run ▾] dropdown.
- HTTP+SSE transport via `codeless-server`. Loopback unauthenticated;
  non-loopback forces `--require-token`.
- Wire types generated via specta into a TS snapshot
  (`crates/codeless-rpc/tests/wire-rpc.ts.snap`) — drift becomes a
  test failure. Generator is `cargo run -p codeless-rpc --example
  wire_ts`; output mirrored into
  `ui/codeless-ui/src/lib/rpc/generated/wire.ts`.

### Runners
- `MockRunner` — scripted, no process spawn. Drives the entire event
  shape without external dependencies. Now only registered when no
  real runner is enabled (see "mock-strip" below).
- `ClaudeRunnerAdapter` — wraps the host `claude` CLI through the
  vendored `ai-runner` crate. Per-job `model` / `permission_mode` /
  `effort` overrides on the wire and on `Job` rows.
- `AnthropicRunnerAdapter` — direct REST against the Anthropic API.
- `CopilotRunnerAdapter` — GitHub Copilot CLI wrapper.
- `TemplateRunner` — orchestrates multi-stage jobs by spinning up a
  per-stage Claude (or per-stage Mock when `--enable-claude` is off)
  and stitching their events into one job.

### Iterate loop (job-as-directory)
- `.codeless/jobs/<name>/` layout with `template.yaml`, `SCOPE.md`,
  `WORKFLOW.md`, plus any other `.md`. Spec pane in the UI lets the
  user edit any of them inline.
- `list_job_files` / `read_job_file` / `write_job_file` /
  `delete_job_file` RPCs.
- Per-stage docs (each stage's `StageSpec` carries its own ordered
  docs list scoped to that stage); the prompt builder folds per-stage
  docs in after the global SCOPE/WORKFLOW.
- Re-run dialog with optional feedback note.
- Handover panel with the structured editor + `write_handover` RPC.

### Stage rollups (landed previous session)
- `StageRecorder` observer subscribes to the bus and persists `Stage`
  + `Task` rows.
- `list_stages` RPC returns `StageRollup` (stage row + cost rollup +
  task count).
- StageTree renders duration + cost per stage. Header shows totals.
- Click a stage → `StageDetail` right-pane view (placeholders for
  the wishlist items: claude session id, commits, tool ribbon, final
  message — see "Next steps" below).

### Tools (Phase 2-5)
- `codeless-tools` crate — shared policy helpers, ported from moxxy
  with attribution in `NOTICE`.
- `http.request` tool with full policy enforcement.
- `browser.*` tool surface backed by a Playwright sidecar.
- `codeless-mcp` — stdio MCP server exposing the tool surface.

### Demo path
- `codeless demo bootstrap` seeds a repo + a mock job (CLI only).
- `codeless demo bootstrap --target-self` registers the inner repo
  as a Claude-runnable target.
- `DOCS/START-SERVER-UI.md` walks the two-terminal-one-browser flow.

## Recently landed on top of master in this session

These have been merged or committed in the workspace:

### Workspace cleanup
- `35414f9 chore: drop legacy job dirs from earlier sessions` —
  removed `kvstore-111916`, `kvstore-112226`, `spec-demo`,
  `stage-rollup-live` from `.codeless/jobs/` so the file tree only
  shows current jobs.
- A pair of revert commits (`0e8cc65` / `e079c9e`) backed out a
  smoke-test job (`bacnet`) that landed during session work; `7453186`
  / `b0937af` did the same for `demo-job`.

### `agent_chat` (user-side, before this session's work)
- New trait method on `RpcServer` for spawning a CLI runner against
  a free-form prompt with a caller-minted `session_id`. Wired through
  to `HttpRpcClient` + the UI side. The runtime impl lives in
  `crates/codeless-runtime/src/rpc.rs:966+`.

## What's in flight on the working tree (uncommitted)

A large cross-layer change is sitting in the working tree, not yet
committed. **Read this entire section before sitting down to commit.**

### 1. `JobStatus::Draft` state + `start_job` RPC

Submit lands jobs in `Draft` by default; the user edits the spec,
then clicks `[run]` to promote `Draft → Queued`. CLI submits keep
the legacy "submit + run" semantics via `start_immediately: true`.

Touchpoints:
- `crates/codeless-types/src/job.rs` — `JobStatus::Draft` variant.
- `crates/codeless-runtime/src/state_machine.rs` — `Draft → Queued`
  + `Draft → Stopped` legal transitions.
- `crates/codeless-runtime/src/store.rs` — encode/decode arms.
- `crates/codeless-rpc/src/methods.rs` — `SubmitJobArgs.start_immediately`,
  new `StartJobArgs`.
- `crates/codeless-rpc/src/server.rs` — `start_job` trait method.
- `crates/codeless-runtime/src/rpc.rs` — `submit_job` lands jobs in
  `Draft` unless `start_immediately`; `start_job` impl emits the
  long-defined-but-never-emitted `Event::JobPromoted`.
- `crates/codeless-server/src/routes.rs` — `/rpc/start_job` route.
- `crates/codeless-client/src/http_client.rs` — HTTP impl.
- `crates/codeless-cli/src/{demo,job,run}.rs` — pass
  `start_immediately: true` to preserve CLI semantics.
- ~22 test files updated to add the new field; one removed test
  pinned obsolete behaviour.

### 2. Submit-time directory scaffolding

`submit_job` parses `template_yaml` if present and, when it conforms
to `JobTemplate`, scaffolds `.codeless/jobs/<name>/template.yaml`,
`SCOPE.md`, and `WORKFLOW.md` *before* the Job row lands. Single
commit `scaffold job: <name>`. The user never has to "promote a
prompt to a template" — every UI submit lands a full spec.

Touchpoints:
- `crates/codeless-runtime/src/rpc.rs` — `seed_job_directory` helper
  + `SCOPE_PRESET` / `WORKFLOW_PRESET` constants. Called from
  `submit_job` only when the YAML parses.
- `crates/codeless-runtime/tests/job_dir_workflow.rs` — replaced two
  obsolete tests (`list_job_files_reports_none_layout_until_first_save`,
  `write_job_file_creates_directory_layout`) with new ones that
  assert the scaffold contract. Removed the flat-layout migration
  test (no longer reachable through normal flow; migration code
  itself stays for legacy DBs).

### 3. Stop publishing `mock` when a real runner is enabled

A `--enable-claude` server now omits `mock` from the published runner
list AND from the factory's `match` arms. `runner: "mock"` from a
stale UI returns `None` from the factory and the driver fails the
job loudly. The boot log's `runners=…` line was also lying;
fixed.

Touchpoints:
- `crates/codeless-cli/src/serve.rs` — three places gated on
  `real_runner_enabled`.

### 4. SubmitJobDialog rebuild

The submit form now collects **name only** (a slug), auto-derives
the branch as `codeless/<slug>`, builds a minimal `template.yaml`,
and sends `start_immediately: false` by default. The "prompt"
textarea is gone. Yellow warning + greyed dropdown when only `mock`
is enabled. Branch input refuses the repo's `default_branch` (and
`main` / `master` defence-in-depth) with an inline red hint and a
disabled submit button.

`PromoteToTemplate.tsx` is **deleted** — never reachable now.
`SpecPane`'s prompt-only branch renders an explainer for legacy
prompt-only jobs (CLI-submitted) telling the user to submit a fresh
one.

Touchpoints:
- `ui/codeless-ui/src/modules/jobs/SubmitJobDialog.tsx` — full rebuild.
- `ui/codeless-ui/src/modules/jobs/spec/SpecPane.tsx` — drop
  PromoteToTemplate import + render branch.
- `ui/codeless-ui/src/modules/jobs/PromoteToTemplate.tsx` — DELETED.

### 5. Three-section JobPage sidebar (SPEC / STAGES / RUN)

Replaces the flat 6-tab strip. Each section has a header label + hint.
Stages section is the spine — clicking a stage replaces the right
pane with `StageDetail`. Header gains `[edit spec]` shortcut and
`[re-run ▾]` dropdown (re-run from scratch live; re-run with feedback
+ from-stage are greyed with explicit "not wired yet" subtext).

Touchpoints:
- `ui/codeless-ui/src/modules/jobs/JobPage.tsx` — full layout rewrite.
- `ui/codeless-ui/src/modules/jobs/StageTree.tsx` — `selectedStageId` /
  `onSelectStage` props (additive, optional).
- `ui/codeless-ui/src/modules/jobs/StageDetail.tsx` — NEW. Right-pane
  detail with placeholder cards for the wishlist items.

### 6. SpecPane redesign

Single vertical scroll. Sections in agent-read order: template.yaml
→ SCOPE.md → WORKFLOW.md → other docs. CodeMirror editing in place
(YAML + markdown modes). Per-section save / discard / dirty marker.
Per-stage docs picker (checkbox list). Global docs-order picker
(reorderable checklist). YAML mutated surgically by
`mutateTemplate.ts` to preserve formatting / comments.

Touchpoints:
- `ui/codeless-ui/src/modules/jobs/spec/SpecPane.tsx` — NEW shell.
- `ui/codeless-ui/src/modules/jobs/spec/TemplateSection.tsx` — NEW.
- `ui/codeless-ui/src/modules/jobs/spec/MarkdownSection.tsx` — NEW.
- `ui/codeless-ui/src/modules/jobs/spec/InlineEditor.tsx` — NEW.
- `ui/codeless-ui/src/modules/jobs/spec/parseTemplate.ts` — NEW
  best-effort summary parser.
- `ui/codeless-ui/src/modules/jobs/spec/mutateTemplate.ts` — NEW
  surgical YAML mutators (`setGlobalDocs`, `setStageDocs`).
- `ui/codeless-ui/src/modules/jobs/JobFilesPane.tsx` — DELETED
  (1305 lines replaced by ~1100 lines split across 4 focused files).

### 7. `useJob` refetchable + JobPage subscribes to lifecycle events

`useJob` now returns `{ ...QueryState<Job>, refetch }`. `JobPage`
subscribes to `job-promoted | job-started | job-completed |
job-failed | job-stopped` SSE events for the focused job and calls
`refetchJob()` on each. `start_job` / `stop_job` also call refetch
on RPC success so the badge updates the moment the click returns.
This fixes "click run, click run again, conflict 409" — the badge
flips to `queued` immediately so the second click is impossible.

`SpecPane` got an `afterSave` helper that re-fetches both
`list_job_files` AND the job row, so editing the YAML and saving
makes the template summary re-render with the new content.

Touchpoints:
- `ui/codeless-ui/src/lib/rpc/hooks.ts` — `useJob` extended.
- `ui/codeless-ui/src/modules/jobs/JobPage.tsx` — subscribe + refetch.
- `ui/codeless-ui/src/modules/jobs/spec/SpecPane.tsx` — afterSave.

### 8. JobsDashboard: fix "new jobs invisible until refresh"

When a `job-queued` event arrives for a job not in the dashboard's
initial `useJobs` snapshot, the handler now calls `get_job` and
seeds the overlay so the row appears immediately. `overlayRef`
mirrors state into a ref so the SSE callback doesn't resubscribe
on every overlay change.

Touchpoints:
- `ui/codeless-ui/src/modules/jobs/JobsDashboard.tsx`.

### 9. Browser shell: drop silent mock fallback

Vite-port auto-fallback removed. `MockRpcClient` only when `?mock=1`.
When the server is unreachable, the browser shows a "cannot reach
codeless server" screen with the URL it tried + a retry button.

`RunMockJobButton` and `NewJobDialog` deleted. Top-level "new job"
button removed; the per-repo `SubmitJobDialog` is the only submit
path.

Touchpoints:
- `ui/codeless-ui/src/shells/browser/main.tsx` — rebuild.
- `ui/codeless-ui/src/modules/jobs/JobsDashboard.tsx` — drop top-level
  + per-repo mock button. CTA copy fixed (the original `repos add`
  command had wrong flag syntax).
- `ui/codeless-ui/src/modules/jobs/RunMockJobButton.tsx` — DELETED.
- `ui/codeless-ui/src/modules/jobs/NewJobDialog.tsx` — DELETED.
- `ui/codeless-ui/src/modules/jobs/JobPage.tsx`,
  `ui/codeless-ui/src/modules/jobs/FilesChanged.tsx` — drop two
  stale "mock runner" strings in error fallbacks.

### 10. Two stale UI strings

`JobPage.tsx` and `FilesChanged.tsx` had error-fallback strings
mentioning "mock runner" that were misleading post-strip. Updated.

## Verified

- `cargo build --workspace` ✓
- `cargo test --workspace` ✓ (76 suites pass)
- `tsc --noEmit` (UI) ✓
- `pnpm build` (UI) ✓
- HTTP smoke: submit → Draft, start_job → Queued, second start_job
  → 409 Conflict. submit with `start_immediately: true` → Queued
  immediately. New job from the dialog appears on the dashboard
  without F5. Editing the YAML in the SPEC pane and saving
  re-renders the summary with the new content.

## Sharp edges right now

### The re-run flow doesn't visibly work

`re-run from scratch` calls `rerun_job` and navigates to the new
job's page. But there's no progress indicator, no SSE-driven
status update, and the dropdown gives no feedback that anything
happened. **This is the next session's primary task — see "Next
steps" below.**

### Wrapper-format YAML at submit time

`codeless job submit <file>.yaml` uses a wrapper YAML format
(`repo: ..., runner: ..., stages: [{name: ...}]`) that doesn't
parse as `JobTemplate`. My submit-time scaffold path detects this
and falls through unscaffolded — backwards-compat preserved, but
the CLI submit path no longer gets the spec scaffold. The right
fix is for the CLI to translate its wrapper format into
`JobTemplate` shape before calling `submit_job`. Lower priority
since the UI is the primary submit surface now.

### `useJob` is fetch-once but used in two places per JobPage

`JobPage` and `SpecPane` each mount their own `useJob(jobId)`. Two
`get_job` calls per page open. Acceptable today; correct fix is a
shared cache (TanStack Query). Each mounts its own SSE subscription
too — same shape.

### Branch-conflict if the user picks `default_branch`

The dialog now refuses, but a stuck job can still happen if you
edit branch via the CLI to something that already has a worktree.
Diagnostic: stuck job will sit `Queued` with `worktree_path: None`
and `started_at: None` indefinitely. Recovery: `stop_job` + start
fresh. The driver should surface allocation failures as
`Event::JobFailed` rather than silently leaving the job queued —
follow-up.

### `agent_chat` is real-time-only, no DB persistence

The user-added `agent_chat` RPC spawns a CLI runner against a
caller-minted session id and emits events. Sessions don't persist
across server restarts. Not in scope here; flagged so future
sessions know.

## Next steps

Priority is driven by the constraint that matters most:
[SCOPE.md "Constraint 2"](./SCOPE.md#why-this-scope-the-key-constraints)
— *the coder loop must run unsupervised for hours.* Today the
substrate (worktrees, SQLite, fresh-session model, handover.md
contract, stage/task/review schema, caps) is in place, but the
mechanics that turn that substrate into "go to bed, wake to commits"
are partial. The four autonomy items (A1–A4) close that gap and
take precedence over UX polish. UX items (U1–U2) follow because
they are what the user *sees* of the autonomy work — re-run with
no feedback is invisible to the operator.

Ordering rationale per item is in its own paragraph; if you disagree
with the order, change it here before starting work so the rationale
travels with the change.

### A1 — Close the handover read/write loop (M-L, blocking everything else)

> Autonomy claim: *"agent reads handover and picks up where the last
> session left off."* Today this is half-built. Without it, "fresh
> session per session" leaks context every tick and 8-hour runs are
> fiction. This is the load-bearing item.

What exists today:

- `write_handover` RPC and the structured editor in `SpecPane` —
  the *operator* can write handover.
- `.codeless/jobs/<name>/` job-as-directory layout, including
  `runs/<name>/handover.md` per [SCOPE.md "Directory and repo
  layout"](./SCOPE.md#directory-and-repo-layout).
- Per-stage docs folded into the prompt by the prompt builder.

What's missing — must land together:

1. **Agent-authored handover at session end.** When a session
   terminates (stage-complete, verify-fail, cap-tripped, crash), the
   runtime synthesises `handover.md` from: the stage's events, the
   commit diff on the branch, the final assistant message
   (`StageDetail` wishlist item #4), and a structured "what halted
   and why" block. This is *not* a Rig helper — it's deterministic
   summarisation over `events` + `git log`. Rig can polish later.
2. **Prompt builder reads handover first.** The next session's
   system prompt opens with handover.md (if present), then SCOPE,
   then WORKFLOW, then the stage's per-stage docs. Order matters:
   handover is the only thing that knows what just happened.
3. **Handover lives on the branch.** Push it with the stage commit
   — per [SCOPE.md:331](./SCOPE.md#L331) "push every stage, never
   defer", the remote *is* the handover state. A session that
   re-clones can still pick up.
4. **Verify by example.** `bacnet`-style smoke tests pass already;
   the real verify is a job that fails stage 2, where session 3
   reads the handover and resumes correctly. Add a demo
   (`handover-resume`) under `codeless/.codeless/jobs/` proving this.

Touchpoints (expected):
- `crates/codeless-runtime/src/handover.rs` — NEW. Deterministic
  synthesiser over events + git diff.
- `crates/codeless-runtime/src/job_driver_loop.rs` — call synthesiser
  at session end; pass handover path into prompt builder.
- `crates/codeless-runtime/src/prompt.rs` (or wherever the prompt
  builder lives) — handover-first ordering.
- `crates/codeless-cli/src/git.rs` (or runtime equivalent) — handover
  goes into the stage commit, not a separate one.

### A2 — Re-run with feedback folded in (S-M, partially scoped)

> Autonomy claim: *"re-run with feedback folded in."* Today the
> dropdown shows it greyed with "needs add_job_note RPC". This is
> the smallest closed-loop control the operator has over a running
> job — without it the only feedback channel is "stop and start
> over."

Wiring (unchanged from the previous snapshot's plan):

- New `add_job_note` Rust RPC + wire types + server route + HTTP impl.
- Notes-folding in `job_driver_loop.rs`'s prompt builder (TODO at
  line ~205). Note becomes a clearly-marked block in the next
  session's prompt, *after* the handover (handover is what happened;
  note is what the operator wants different).
- UI: textarea in the re-run dropdown → `add_job_note` then
  `rerun_job`. Re-run navigates to the new job's RunPane.
- MCP parity: `codeless.job.rerun` carries an optional `note` field
  per [AGENT-CONTROL-PLANE.md](./AGENT-CONTROL-PLANE.md) Slice 2.

Sequencing: ship the RPC + folding *before* the UI wires it. CLI
gets `codeless job rerun <id> --note "..."` on the same change.

### A3 — Verify-fail policy: agent decides retry vs. escalate (M)

> Autonomy claim: *"verify-fails-loop: agent decides retry vs.
> escalate."* Today a failed stage halts the job. For long runs
> this is too brittle — one flaky test stops the night. The fix is
> declarative policy on the stage, not agent-side decisioning
> (agent-side decisioning is unbounded and burns budget).

Design:

- `StageSpec` gains `on_verify_fail`:
  - `retry_with_feedback` (max N, default 2) — synthesise a
    feedback note from the verify output and fire a fresh session
    on the same stage. Counts against the job's wall-clock + cost
    caps.
  - `escalate_to_review` — materialise a `Review` row, halt the
    stage in `awaiting-review`, emit `review-requested`. This is
    where the operator's morning queue comes from.
  - `fail_stage` (today's behaviour, kept as the default for
    backward compat).
- Verify output flows into the synthesised feedback note via the
  same handover-builder pipeline (A1). Don't write a second
  summariser.
- Per-job cap is the floor: retries inside a stage cannot exceed
  the job's cost/wall-clock budget. Cap-tripped wins over retry.

Touchpoints:
- `crates/codeless-types/src/stage.rs` — `OnVerifyFail` enum.
- `crates/codeless-rpc/src/methods.rs` — wire type.
- `crates/codeless-runtime/src/state_machine.rs` — new transitions
  (`verify-failed → running` for retry; `verify-failed →
  awaiting-review` for escalate).
- `crates/codeless-runtime/src/job_driver_loop.rs` — policy
  evaluation.
- UI: `StageDetail` shows retry count + policy; spec pane lets the
  user pick policy per stage.

Order: ship A1 first. The retry path *is* the handover path with
a synthesised note — it cannot work cleanly until handover does.

### A4 — Loop-level aggregate cost ceiling (S, currently post-MVP)

> SCOPE.md:379 calls this out as the highest-priority budget feature
> after per-job caps. Twenty jobs × per-job cap is the surprise
> bill on waking. Pull it forward to land alongside A3 — both are
> cheap, both are *required* for unattended use.

Design:

- Two-tier ceiling per [SCOPE.md "Loop-level aggregate ceilings"](./SCOPE.md#cost-visible-and-cappable-from-day-one):
  `Continue` / `Escalate` / `Halt`.
- Evaluated by the runtime *before firing each session* — at the
  scheduler tick, not inside a runner. Cheaper than per-token
  enforcement and matches how per-job caps already work.
- Config in `~/.config/codeless/limits.toml` (next to auth.toml).
  Defaults: warn at $20/day cumulative, halt at $50/day. User
  edits or disables freely.
- `Escalate` materialises a job-level `Review` row; the loop pauses
  until the operator clears it.

Touchpoints:
- `crates/codeless-types/src/limits.rs` — NEW.
- `crates/codeless-runtime/src/scheduler.rs` — pre-fire check.
- `crates/codeless-server/src/routes.rs` — `get_limits`,
  `set_limits` RPCs.
- UI: dashboard header shows cumulative-vs-ceiling bar.

### U1 — Re-run + RUN page UX overhaul (M-L, depends on A1+A2)

What the user originally asked for in this session. Deferred behind
A1 + A2 because: without handover and `add_job_note`, the RUN page
is just *animations over a still-broken loop*. With them, the RUN
page becomes the place where you watch unattended work resume,
which is the actual product.

Approach (unchanged from the previous snapshot):

- Build a `RunPane` (replaces today's Timeline section as the RUN
  default landing). Live SSE-driven view of the job's lifecycle:
  Draft → Queued → Running → (Stages flowing in) → Completed.
- Visual progress between states: animated lines connecting status
  nodes, framer-motion (`motion/react`) transitions when a stage
  flips state.
- `[run]` and `[re-run ▾]` move to the RunPane. Header keeps the
  badge + cost/time totals only.
- Re-run navigates to the new job's RunPane, which immediately
  shows "starting…" state.
- Once A1 + A2 land, the RunPane also surfaces the most recent
  handover excerpt and the operator's last feedback note inline,
  so resuming a job tells you *why* this session is firing.

### U2 — Wishlist items for `StageDetail` (M, parallelisable with A1)

Placeholder cards exist; wire the data. These compose with A1 —
the handover synthesiser reads commits, final messages, and tool
calls anyway, so the data plumbing is shared.

1. **Claude session ID** per stage. `RunResult.session_id` flows
   out of ai-runner already; capture on `Stage` row + new event.
2. **Per-stage commits.** `git log <branch>` joined to stage
   timestamps.
3. **Tool-call ribbon.** Roll up `Event::ToolCall` per stage at
   query time (no schema change needed).
4. **Final assistant message excerpt.** Buffered in the claude
   adapter today; needs an event or persistence.

### A5 — Planner: goal → stages (L, deliberately last)

> Autonomy claim: *"planner generates stages from a goal."* The
> aspiration in [SCOPE.md:611](./SCOPE.md#L611) is correct but
> deliberately scheduled last:
>
> - Per the same SCOPE entry, "defer until ~10 real jobs exist to
>   train against — a bad plan poisons the whole job."
> - Per [SCOPE.md "Helper role" hard rule #1](./SCOPE.md#helper-role--rig-optional-never-gates-a-job),
>   the loop must work end-to-end with zero helpers configured.
>   Planner is a helper, not a gate.
> - Without A1 (handover) and A3 (verify-fail policy), an
>   auto-generated plan failing mid-way leaves the operator with no
>   recovery surface anyway.

Land it after A1–A4 have produced a corpus of real `runs/*/log.md`
that a Rig planner can RAG against. First version: take user goal +
the repo's `CODELESS.md`, emit a `template.yaml` draft, *user
reviews before submit*. No auto-submit, ever.

### Smaller / housekeeping items (any time)

These are real but they don't move the autonomy needle. Pick them
up when the autonomy track is between phases.

- **Re-run from a specific stage (L).** Needs the Job/Run schema
  split (JOB-WORKFLOW.md Phase B). Bigger than a single session.
  Park until there's a real driver — most likely once the morning
  review queue exists and the user wants "redo stage 3 with this
  note" as a verb.
- **CLI submit translates wrapper YAML → JobTemplate.** So
  `codeless job submit` jobs also get the disk scaffold. Small
  focused change in `crates/codeless-cli/src/job.rs`. Lower
  priority since the UI is the primary submit surface now.
- **Driver surfaces worktree-allocation failures (S).** Today a
  job whose `default_branch` is taken sits silently in `Queued`.
  Should emit `Event::JobFailed` with a clear reason. Becomes more
  urgent once unattended runs are real — a stuck-queued job at 3am
  is invisible.
- **`agent_chat` persistence.** Real-time-only today, no DB
  persistence. Not in scope for the autonomy track; flag if a user
  starts relying on it.

## Documentation that's now stale (housekeeping)

- `DOCS/JOB-DIR-KICKOFF.md` — references the destroyed session and a
  rebuild plan that's now done. Delete or mark "historical".
- `DOCS/JOB-DIR.md` "What this rebuilds" section at the bottom is
  obsolete.
- `DEMO-UI.md` `ux-12`–`ux-15` entries describe an iterate-loop UI
  that's been substantially rebuilt twice since.
- This snapshot supersedes any older `DOCS/sessions/` notes.

## TL;DR for the next session

1. Read this doc, especially the **Next steps** section. The
   ordering has changed: autonomy (A1–A4) now precedes the RUN
   page UX overhaul (U1), because the UX overhaul is animation
   over a still-broken loop until handover + re-run-with-feedback
   land.
2. The user's working tree is full of uncommitted cross-layer work
   (Draft state + scaffolding + UI rebuild). Decide whether to
   commit it as-is (one big "submit-flow overhaul" commit) or
   split into the 10 numbered chunks above. Don't push yet without
   explicit user OK — two pairs of revert commits in recent
   history show the user wants control over what lands.
3. **Start on A1 — close the handover read/write loop.** Without
   it, "fresh session per session" leaks context every tick and the
   8-hour-unattended use case stays fiction. Verify with a real
   demo job under `codeless/.codeless/jobs/handover-resume/` where
   stage 2 fails and stage 3's session resumes from handover.
4. A2 (`add_job_note` + folding) is the smallest valuable next
   change after A1 and unblocks U1. A3 (verify-fail policy) and
   A4 (loop-level cost ceiling) round out the autonomy track.
5. U1 (RUN page UX) and U2 (StageDetail wishlist) follow once
   A1–A2 are in. U2 can run in parallel with A1 since the
   handover synthesiser reads the same data the wishlist cards
   render.
