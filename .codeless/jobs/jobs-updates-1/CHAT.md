# Chat for this job

## user @ 2026-05-14T22:58:03Z

confirm you undertsanr the scope

## assistant @ 2026-05-14T22:58:18Z

Yes — I understand the scope. Quick read-back so we're aligned:

**Goal:** Implement the job page UX redesign from `DOCS/JOB-UI.md` — tabs (`CHAT` / `SPEC` / `Stages` / `Stage-<N>`), a scannable stage overview with inline ticks + tests, and a live per-stage chat that talks to the warm session.

**Why now:** the current job page has no stage overview, no per-stage live chat, and no clean way to restart a failed gate without losing agent context.

**Shape of the work (6 stages):**
1. `schema` — additive `stage.goal`, `stage.acceptance`, `verify: Vec<VerifyStep>` (with `verify_cmd` sugar), and `session_idle_timeout` on the job in `codeless-types`.
2. `events` — `verify-step-passed` / `verify-step-failed` variants emitted per step by `codeless-runtime`.
3. `idle-timeout` — runtime honours `session_idle_timeout`; transparent archive-then-handover on the next message past timeout.
4. `ui-overview` — `Stages` tab rendering stages + inline ticks/tests + `[ restart ▾ ]`.
5. `ui-stage-detail` — `Stage-<N>` tab with goal/gates/failure summary and `rerun now` / `new session + handover` / `stop`.
6. `ui-stage-chat` — live chat per stage, wired through `--continue <session_id>`, reattaches within idle timeout.

**Load-bearing constraints:**
- SCOPE hard rule #1 (as rewritten): warm session on interactive resumption; fresh session only on autonomous advancement; `new session + handover` is an explicit user action.
- R2 (RpcClient only), R3 (no per-shell UI files), R4 (SQLite is truth).
- All three gates (test / clippy -D warnings / fmt --check) green per commit.

**Current spec state:** SCOPE.md and WORKFLOW.md are filled in and coherent; `template.yaml` is still the bootstrap stub (`goal: TODO`, one TODO stage). The six stages from WORKFLOW.md aren't in `template.yaml` yet, and there are no per-stage `*.md` files referenced.

Want me to (a) flesh out `template.yaml` with the six stages + goal, and (b) draft per-stage `*.md` files for each? Or keep the spec lean and let each stage's prompt rely on SCOPE/WORKFLOW alone?
