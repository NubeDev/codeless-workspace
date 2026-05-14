# Scope — Goals 4 and 5 (message plane + host-routing)

The source of truth for hackline's design lives **inside this repo**.
This job does not restate any of it; the agent must read the docs
listed below before writing code.

## Where the truth lives — read these first

| Topic | File |
|---|---|
| Top-level scope, all phases, all open questions | [`SCOPE.md`](../../../SCOPE.md) |
| Architecture overview | [`DOCS/ARCHITECTURE.md`](../../../DOCS/ARCHITECTURE.md) |
| Wire surface (REST + SSE + keyexprs) | [`SCOPE.md` §5](../../../SCOPE.md) |
| Persistence (tables, ring buffers) | [`SCOPE.md` §7](../../../SCOPE.md), [`DOCS/DATABASE.md`](../../../DOCS/DATABASE.md) |
| Auth model (already implemented) | [`DOCS/AUTH.md`](../../../DOCS/AUTH.md) |
| Existing REST surface and openapi | [`DOCS/REST-API.md`](../../../DOCS/REST-API.md), [`DOCS/openapi.yaml`](../../../DOCS/openapi.yaml) |
| CLI (existing) | [`DOCS/CLI.md`](../../../DOCS/CLI.md) |
| Phasing roadmap (the phases this job covers) | [`SCOPE.md` §13](../../../SCOPE.md) |

## Prior work — read the session notes for context

Goals 0–3 are already done. Each session note is the plan-table +
outcome for one goal — they show the pattern this job follows:

- [`DOCS/sessions/2026-05-14-goal0-bridge-spike.md`](../../../DOCS/sessions/2026-05-14-goal0-bridge-spike.md) — Zenoh spike (Phase 0)
- [`DOCS/sessions/2026-05-14-goal1-real-binaries.md`](../../../DOCS/sessions/2026-05-14-goal1-real-binaries.md) — agent + gateway scaffolds (Phase 1 start)
- [`DOCS/sessions/2026-05-14-goal2-sqlite-rest.md`](../../../DOCS/sessions/2026-05-14-goal2-sqlite-rest.md) — SQLite + REST CRUD (Phase 1 middle)
- [`DOCS/sessions/2026-05-14-goal3-auth-cli.md`](../../../DOCS/sessions/2026-05-14-goal3-auth-cli.md) — auth + CLI completion (Phase 1 done)

## What this job delivers

Two stages, each adds one new session note under `DOCS/sessions/` and
the code to back it. The stages map onto SCOPE.md §13 in order:

- **Stage 1 — Goal 4: message plane (events + logs).** Implements
  SCOPE.md §13 "Phase 1.5". Demoable end-state: a device-side
  `publish_event` / `publish_log` round-trips through the gateway's
  fan-in subscribers, lands in the `events` / `logs` tables with
  ring-buffer pruning, and is readable via `GET /v1/events`,
  `GET /v1/log`, and the corresponding SSE streams.
- **Stage 2 — Goal 5: commands + api + HTTP host-routing.**
  Implements SCOPE.md §13 "Phase 2". Demoable end-state: durable
  `cmd_outbox` with ack semantics, synchronous `api` round-trip,
  and the axum HTTP front-end routing `device-<id>.cloud.example.com`
  to the right tunnel.

## Constraints (non-negotiable)

- No drift between code and SCOPE.md. If a design needs to change,
  update SCOPE.md in the same commit.
- Each stage produces its own `DOCS/sessions/2026-05-14-goalN-*.md`
  with the plan table (Step / Status), Outcome, and Design sections
  in the same shape as `goal3-auth-cli.md`.
- `cargo check --workspace` and `cargo test --workspace` must be
  green at the end of each stage. Pre-existing dead-code warnings
  in `hackline-agent` are tolerated; new warnings are not.
- No work outside the hackline crates and `DOCS/sessions/`. Do not
  touch `SCOPE.md` unless the design genuinely needs to change.
- All work happens on this job's current branch in-place (in_repo
  mode); commit per stage with a clear message.

## Out of scope

- Phase 3 (admin UI, audit completeness) and later — explicitly
  later stages.
- Cross-org isolation (Phase 4).
- Postgres backend, ACME, npm package (Phase 5).
- Any change to the auth layer beyond enforcing existing middleware
  on the new routes.
