# Workflow — jobs-updates-1

How the agent drives this job. Stages run in the order listed in
`template.yaml`. Each stage ends at its `verify` gates; advancing to
the next stage requires every gate green.

## Stage order and why

1. **`schema`** — additive wire-type changes. Everything else
   depends on the new fields existing and round-tripping.
2. **`events`** — once stages can carry layered `verify` steps, the
   runtime emits one event per step. The UI's per-gate status glyphs
   read these.
3. **`idle-timeout`** — backend honours `session_idle_timeout`. The
   UI's "did this just transparently fall back to a fresh session?"
   indicator needs this signal to be real, not faked.
4. **`ui-overview`** — `Stages` tab. Pure render over events emitted
   by stages 1–3.
5. **`ui-stage-detail`** — `Stage-<N>` tab. Buttons wired, no chat
   yet.
6. **`ui-stage-chat`** — live chat against the warm session. This is
   the payoff stage and must come last because every other piece is
   a prerequisite.

## Sessions across stage boundaries

Per [`SCOPE.md` hard rule #1](../../../DOCS/SCOPE.md#hard-rules-for-the-coding-runner)
(as rewritten by this job's prep work):

- **Autonomous advancement (stage N → stage N+1, verify green)**:
  fresh session, fresh agent re-onboards from `handover.md`.
- **Interactive resumption on a failed / halted / paused stage**:
  same session continues via `--continue <session_id>`. The user
  talking to the agent is the same agent that just ran.
- **`new session + handover`** is an explicit user action on a
  failed gate. Codeless does not pick it silently.

The job is allowed to halt with `[!]` on any stage and wait for the
operator. Partial implementations are not allowed; if a stage cannot
finish, mark it `[!]` and stop.

## Verify policy

Each stage's `verify:` list runs in declaration order. The stage
passes only when every step passes. On first failure, later steps
emit the `skipped` lifecycle variant — they are not silently absent.

A failed verify halts the stage by default. Per SCOPE
[A3](../../../DOCS/PROGRESS.md#a3--verify-fail-policy-agent-decides-retry-vs-escalate-m),
a stage may opt into a bounded retry-with-feedback (cap counted
against the job's cost + wall-clock fuses) — none of the stages in
this job opt in. All failures surface to `handover.md` and the
operator.

## Commits and pushes

Per [`JOB-LOOP.md`](../../../DOCS/JOB-LOOP.md), commit and push via
mani from the workspace root, never raw git:

```sh
./bin/mani --config mani.yaml run commit --projects codeless \
  MSG='stage N: <stage-name>'
./bin/mani --config mani.yaml run push --projects codeless
```

One logical batch per tick. No `--force`, no `--no-verify`. If a
hook fails, fix the cause.

## Code touchpoints

Where each stage will land work. These are pointers, not contracts —
the implementing agent owns the final paths.

| Stage | Crate / package | Notes |
|---|---|---|
| `schema` | [`codeless/crates/codeless-types/`](../../../codeless/crates/codeless-types/) | Add the three fields; bump migration. |
| `events` | [`codeless/crates/codeless-types/`](../../../codeless/crates/codeless-types/), [`codeless/crates/codeless-runtime/`](../../../codeless/crates/codeless-runtime/) | New event variants + emit sites. |
| `idle-timeout` | [`codeless/crates/codeless-runtime/`](../../../codeless/crates/codeless-runtime/) | Session resumption path. |
| `ui-overview` | [`codeless/ui/codeless-ui/src/`](../../../codeless/ui/codeless-ui/src/) | New components; subscribe via existing `RpcClient` hooks. |
| `ui-stage-detail` | same | Tab component + RPC wiring for the three buttons. |
| `ui-stage-chat` | same | `useEventStream` per stage, `--continue` semantics. |

## Done

This job is done when:

- All seven stages green (six `verify`-gated, one prep already
  landed in DOCS).
- `template.yaml` is the canonical example of the new fields in use.
- A user can open the job page, scan stages + ticks, restart a
  failed gate, and chat with the stage's warm session.
- `DOCS/JOB-UI.md` matches what shipped (update the doc if reality
  diverges; the doc is not aspirational).
