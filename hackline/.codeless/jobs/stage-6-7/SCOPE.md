# Scope - Goals 6 and 7 (audit + admin UI, multi-tenant orgs)

The source of truth for hackline's design lives **inside this repo**.
This job does not restate any of it; the agent must read the docs
listed below before writing code.

## Where the truth lives - read these first

| Topic | File |
|---|---|
| Top-level scope, all phases, all open questions | [`SCOPE.md`](../../../SCOPE.md) |
| Architecture overview | [`DOCS/ARCHITECTURE.md`](../../../DOCS/ARCHITECTURE.md) |
| Wire surface (REST + SSE + keyexprs) | [`SCOPE.md` §5](../../../SCOPE.md), [`DOCS/KEYEXPRS.md`](../../../DOCS/KEYEXPRS.md), [`DOCS/REST-API.md`](../../../DOCS/REST-API.md) |
| Persistence (tables, audit actions, retention) | [`SCOPE.md` §7](../../../SCOPE.md), [`DOCS/DATABASE.md`](../../../DOCS/DATABASE.md) |
| Auth model (claim + scoped tokens) | [`SCOPE.md` §6](../../../SCOPE.md), [`DOCS/AUTH.md`](../../../DOCS/AUTH.md) |
| Observability (metrics, health) | [`SCOPE.md` §10](../../../SCOPE.md) |
| Configuration | [`SCOPE.md` §11](../../../SCOPE.md), [`DOCS/CONFIG.md`](../../../DOCS/CONFIG.md) |
| CLI (existing) | [`DOCS/CLI.md`](../../../DOCS/CLI.md) |
| Phasing roadmap (the phases this job covers) | [`SCOPE.md` §13](../../../SCOPE.md) |

## Prior work - read the session notes for context

Goals 0-5 are already done. Each session note is the plan-table +
outcome for one goal - they show the pattern this job follows:

- [`DOCS/sessions/2026-05-14-goal0-bridge-spike.md`](../../../DOCS/sessions/2026-05-14-goal0-bridge-spike.md) - Zenoh spike (Phase 0)
- [`DOCS/sessions/2026-05-14-goal1-real-binaries.md`](../../../DOCS/sessions/2026-05-14-goal1-real-binaries.md) - agent + gateway scaffolds (Phase 1 start)
- [`DOCS/sessions/2026-05-14-goal2-sqlite-rest.md`](../../../DOCS/sessions/2026-05-14-goal2-sqlite-rest.md) - SQLite + REST CRUD (Phase 1 middle)
- [`DOCS/sessions/2026-05-14-goal3-auth-cli.md`](../../../DOCS/sessions/2026-05-14-goal3-auth-cli.md) - auth + CLI completion (Phase 1 done)
- [`DOCS/sessions/2026-05-14-goal4-message-plane.md`](../../../DOCS/sessions/2026-05-14-goal4-message-plane.md) - events + logs (Phase 1.5)
- [`DOCS/sessions/2026-05-14-goal5-cmd-api-host-routing.md`](../../../DOCS/sessions/2026-05-14-goal5-cmd-api-host-routing.md) - cmd + api + host-routing (Phase 2)

## What this job delivers

Two stages, each adds one new session note under `DOCS/sessions/` and
the code to back it. The stages map onto SCOPE.md §13 in order:

- **Stage 1 - Goal 6: audit completeness + admin UI.** Implements
  SCOPE.md §13 "Phase 3". Demoable end-state:
  - Every bridged TCP connection (tunnel plane) writes a single
    `tunnel.session` audit row at open and updates it at close with
    `bytes_up`, `bytes_down`, and `duration_ms`.
  - `cmd.send`, `cmd.cancel`, and `api.call` audit entries fire from
    the corresponding REST handlers, with the required `detail` keys
    from SCOPE.md §7.2.
  - `GET /metrics` exposes the Prometheus counters / gauges listed in
    SCOPE.md §10.2 (admin-token gated in v0.1).
  - A static React admin bundle ships with the gateway and renders
    live tunnels, the cmd outbox, and the live events stream against
    the existing REST + SSE surface. No new wire surface beyond the
    `/metrics` endpoint and whatever the admin bundle needs that
    SCOPE.md §5.3 already lists.
- **Stage 2 - Goal 7: multi-tenant orgs.** Implements SCOPE.md §13
  "Phase 4". Demoable end-state:
  - `orgs` table; `org_id` non-null FK on `users` and `devices`;
    bootstrap claim flow inserts the first org and stamps the owner
    user + any created devices with that `org_id`.
  - Cross-org isolation enforced at every REST handler that scopes by
    device or user: no row from another org is ever returned, mutated,
    or referenced. Audit + events + logs + cmd_outbox all gain
    org-aware queries.
  - Zenoh keyexpr prefix per org (`hackline/<org_slug>/<zid>/...`)
    plus matching Zenoh ACL grants, so a device in org A cannot reach
    org B's gateway subscribers even if it tries.
  - `hackline org` CLI subcommands for create / list / inspect, plus
    `hackline login` capturing the org context so subsequent calls
    are scoped correctly.

## Constraints (non-negotiable)

- No drift between code and SCOPE.md. If a design needs to change,
  update SCOPE.md in the same commit.
- Each stage produces its own `DOCS/sessions/2026-05-14-goalN-*.md`
  with the plan table (Step / Status), Outcome, and Design sections
  in the same shape as `goal5-cmd-api-host-routing.md`.
- `cargo check --workspace` and `cargo test --workspace` must be
  green at the end of each stage. Pre-existing dead-code warnings
  in `hackline-agent` are tolerated; new warnings are not.
- No work outside the hackline crates and `DOCS/sessions/`. Do not
  touch `SCOPE.md` unless the design genuinely needs to change.
- All work happens on this job's current branch in-place (in_repo
  mode); commit per stage with a clear message.
- Stage 1 must merge cleanly before Stage 2 starts. Stage 2 touches
  every table Stage 1 writes audit rows into, so running them in
  parallel guarantees rebase pain. See WORKFLOW.md.

## Out of scope

- Phase 5 (ACME in-gateway, Postgres, npm package, Rust→TS codegen)
  and any later phase.
- Per-org branding of `device-N.<org>.cloud.example.com` (SCOPE.md
  §14 Q5) - the keyexpr prefix and DB row land here, the wildcard
  cert / DNS plumbing is Phase 5.
- Replacing the React admin bundle's tooling (build system, design
  system). Ship the smallest viable static bundle; do not introduce
  a new framework.
- Any change to the auth layer beyond adding `org_id` to the
  `AuthedUser` extractor and threading it through.
- ESP32 / constrained-device support (SCOPE.md §3.7) beyond what
  the existing `devices.class` column already records.
