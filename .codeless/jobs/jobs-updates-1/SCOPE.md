# Scope — jobs-updates-1

Land the **job page UX redesign** described in
[`../../../DOCS/JOB-UI.md`](../../../DOCS/JOB-UI.md): one job page,
tabs for navigation (`CHAT` / `SPEC` / `Stages` / `Stage-<N>`),
stage-overview with inline ticks + tests, and a live chat per stage
that talks to the still-warm session.

This job exists because the current job page has no stage overview
worth scanning, no per-stage live chat, and no way to restart a
failed gate without losing the agent's context. The fix is partly
schema (three additive fields on `stage`) and partly UI (tabs +
overview + stage detail + live chat).

## Success looks like

- A user opens a job and sees every stage with its ticks and tests
  inline as a single scannable column.
- A failed gate shows a `[ restart ▾ ]` with two options:
  **`rerun now`** (continue the warm session) and
  **`new session + handover`** (archive + fresh agent).
- Opening a `Stage-<N>` tab shows the stage's goal, gates, failure
  summary, and a live chat input that streams the agent's reply.
- Closing and reopening a stage tab re-attaches to the same session
  (within `session_idle_timeout`); after the timeout, the next
  message transparently becomes a `new session + handover`.
- The `Stages` / `SPEC` / `CHAT` tabs always exist; `Stage-<N>` tabs
  are user-opened, pinnable, and survive reload.

## In scope

- **SCOPE doc edits** (done before this job started) — hard rule #1
  rewritten to distinguish autonomous advancement from interactive
  resumption. See [`../../../DOCS/SCOPE.md`](../../../DOCS/SCOPE.md)
  "Hard rules for the coding runner".
- **`DOCS/JOB-UI.md`** (done) — the spec this job implements.
- **Schema additions** (this job — first stage):
  - `stage.goal: String`
  - `stage.acceptance: Vec<String>`
  - `verify: Vec<VerifyStep>` (sugar: a bare `verify_cmd: String`
    still parses as a single-step list)
  - `session_idle_timeout` on the job (default 30 min)
- **UI additions** (this job — later stages):
  - Tabs container on the job page.
  - `Stages` overview tab with inline ticks + tests + `[ restart ▾ ]`.
  - `Stage-<N>` detail tab with goal / gates / failure summary +
    three buttons (`rerun now`, `new session + handover`, `stop`).
  - Live stage chat wired to `--continue <session_id>`.
- **Runtime additions**:
  - `session_idle_timeout` enforcement + transparent archive-then-
    handover on the next message.
  - `verify-step-passed` / `verify-step-failed` events emitted per
    step.

## Out of scope

- Multi-job dashboard / cross-job overview.
- `CHAT` tab planner-backed agent (separate Rig-helper track).
- File-tree / editor surfaces around the job page.
- Mobile-shell-specific layout adjustments.
- Phase 7 multi-tenant auth.

## Constraints

- Honour SCOPE hard rule #1 as rewritten — fresh session only at
  *autonomous* stage advancement; interactive resumption continues
  the warm session.
- Honour R2 — UI imports only `RpcClient`, no direct `@tauri-apps/*`.
- Honour R3 — no per-shell UI files; one component tree, responsive.
- Honour R4 — SQLite is the source of truth for stage / tick state;
  UI does not maintain a parallel store.
- All three gates green before each commit:
  `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`.

## Deliverables

- Three new fields on the stage schema with migrations + serde
  round-trip tests in `codeless-types`.
- `verify-step-passed` / `verify-step-failed` event variants in
  `codeless-types`, emitted by `codeless-runtime`.
- `session_idle_timeout` honoured by the session-resumption path in
  `codeless-runtime`.
- New UI components for tabs / stage overview / stage detail / stage
  chat under `codeless/ui/codeless-ui/src/` (paths chosen by the
  agent implementing the stage).
- Updated `template.yaml` example using the new fields.
- This file kept up to date as stages land.

## Links

- Spec this job implements: [`DOCS/JOB-UI.md`](../../../DOCS/JOB-UI.md)
- Authoritative project scope: [`DOCS/SCOPE.md`](../../../DOCS/SCOPE.md)
- Hard rule #1 (warm session vs. reset): [`DOCS/SCOPE.md` "Hard rules for the coding runner"](../../../DOCS/SCOPE.md#hard-rules-for-the-coding-runner)
- Job page screenshot reference (text mock): [`DOCS/JOB-UI.md` "The `Stages` tab"](../../../DOCS/JOB-UI.md#the-stages-tab--the-overview)
- Existing run-log primitives the UI builds on: [`DOCS/JOBS-UX.md`](../../../DOCS/JOBS-UX.md)
- UI architecture (single codebase, four shells): [`DOCS/UI-ARCHITECTURE.md`](../../../DOCS/UI-ARCHITECTURE.md)
- Per-repo memory: [`codeless/CODELESS.md`](../../../codeless/CODELESS.md)
