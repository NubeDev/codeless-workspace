# Workflow

How to drive Goal 6 and Goal 7. The shape mirrors how Goals 0-5 ran
(see `DOCS/sessions/2026-05-14-goal5-cmd-api-host-routing.md` for
the most recent gold example).

## Before any stage starts

1. Read [`../../../SCOPE.md`](../../../SCOPE.md) end-to-end. Pay
   particular attention to §3 (architecture), §5 (wire), §7
   (persistence and well-known `audit.action` values), §10
   (observability), and §13 (phasing).
2. Read every `DOCS/sessions/2026-05-14-goal*.md` to understand what
   landed already and the conventions used (plan table, Outcome,
   Design sections).
3. Skim the existing crates so you know where things live:
   `crates/hackline-gateway/`, `crates/hackline-agent/`,
   `crates/hackline-client/`, `crates/hackline-proto/`,
   `crates/hackline-cli/`.

## Per-stage protocol (applies to Goal 6 and Goal 7)

1. **Plan the stage**. Open the session note file for the stage
   (`DOCS/sessions/2026-05-14-goal6-audit-admin-ui.md` for stage 1,
   `DOCS/sessions/2026-05-14-goal7-multi-tenant-orgs.md` for stage 2).
   Write the plan table first - every row marked `[ ]`. Commit the
   empty plan as the first commit of the stage.
2. **Implement one step at a time**, ticking `[ ]` → `[x]` in the
   session note as each step lands. Commit per step or per logical
   batch; commit messages start with `goal6: ` or `goal7: `.
3. **Verify between steps** with `cargo check --workspace`. Run
   `cargo test --workspace` after any persistence, router, SDK, or
   migration change.
4. **Write the Outcome + Design sections** of the session note
   before closing the stage. Outcome lists what was verified (curl,
   netcat, integration tests, screenshots of the admin UI for
   Goal 6). Design captures the why of the non-obvious decisions,
   especially anything that pushed back on SCOPE.md.
5. **End-of-stage gate**:
   - `cargo check --workspace` clean (no new warnings).
   - `cargo test --workspace` passes.
   - Session note has every row `[x]` plus Outcome and Design.
   - Manual demo command(s) in the Outcome work copy-pasted.

## Stage-specific shape

### Stage 1 - Goal 6 (audit + admin UI)

Suggested step ordering (the agent owns the final plan table):

1. `tunnel.session` audit row: open at bridge start, finalise at
   bridge close with byte counters wired into the existing
   `bridge::initiate_bridge` path. Integration test: bridge a TCP
   echo, assert one row with non-zero `bytes_up` / `bytes_down`.
2. `cmd.send` / `cmd.cancel` / `api.call` audit emit, with the
   exact `detail` keys from SCOPE.md §7.2's well-known table.
3. `GET /metrics` Prometheus text endpoint, admin-token gated,
   covering every counter / gauge in SCOPE.md §10.2.
4. Static React admin bundle under
   `crates/hackline-gateway/static/admin/` (or equivalent), served
   under `/admin` with a long-cache header on hashed asset paths.
   Views: device list, live tunnels, cmd outbox, live events
   stream. No new REST routes; the bundle uses the existing
   `/v1/*` and `/v1/*/stream` surfaces.
5. Session note Outcome + Design.

### Stage 2 - Goal 7 (multi-tenant orgs)

Suggested step ordering:

1. Migration: `orgs` table; non-null `org_id` FK added to `users`
   and `devices`; backfill existing rows into a single default org
   created in the same migration. New migration only - never edit
   a landed one.
2. Auth: `AuthedUser` carries `org_id`; every REST handler that
   touches devices, tunnels, cmd_outbox, events, logs, or audit
   filters by it. The `auth::scope` helpers grow an org check
   that runs **before** the per-device customer check from Goal 5.
3. Zenoh keyexpr prefix per org. Gateway fan-in subscribers and
   bridge queryables move from `hackline/<zid>/...` to
   `hackline/<org_slug>/<zid>/...`. ACL config grows a per-org
   grant template. Devices joining the fabric pick up the new
   prefix from their config; the migration sets `org_slug` on
   the default org from existing gateway config.
4. Claim flow + CLI: first-boot claim creates the first org; the
   owner user inherits it. `hackline org create / list / inspect`
   and `hackline login` carrying the org context.
5. Integration test: two orgs, one device each, cross-org REST
   call returns `404 not_found` (not `403`, per usual leak-the-
   minimum convention - decide in Design and write down which).
6. Session note Outcome + Design.

## SCOPE.md drift

If during the stage the right design diverges from SCOPE.md, **stop
and update SCOPE.md in the same commit** that introduces the change.
Do not let the code outrun the doc; every prior goal called this
rule out and it has held.

Goal 7 in particular touches a phase that SCOPE.md only sketches
(§13 Phase 4 is two bullet points). The agent is expected to write
the missing detail into SCOPE.md §5, §6, §7 as it lands - the
sketch becomes a spec in the same commit.

## Out-of-band rules

- Never push to `origin`. The remote is wrong for this checkout -
  commits stay local until the operator pushes.
- Never run `git rebase`, `git reset --hard`, or `git push --force`.
- Do not modify the auth layer (`crates/hackline-gateway/src/auth/`)
  beyond adding `org_id` to `AuthedUser` (Goal 7) and adding new
  routes' middleware wiring.
- New REST routes must be protected by the existing `AuthedUser`
  extractor unless they belong to the unauthenticated set defined
  in `DOCS/AUTH.md` (health, claim/status, claim). `/metrics` uses
  the admin-token gate, not anonymous access, in v0.1.
- New tables and column additions go in a fresh `Vnnn__*.sql`
  migration file under `crates/hackline-gateway/migrations/`;
  never edit a landed migration.
- Ring-buffer pruning continues to run inside the same transaction
  as the insert, per SCOPE.md §7.
- Stage 2 must not start until Stage 1 has merged. The two stages
  touch overlapping rows (audit + cmd_outbox + events) and running
  them in parallel guarantees a painful rebase.

## Definition of done for the whole job

- `DOCS/sessions/2026-05-14-goal6-*.md` and
  `DOCS/sessions/2026-05-14-goal7-*.md` both exist and are complete.
- Every well-known `audit.action` in SCOPE.md §7.2 is emitted by
  the corresponding handler, verified by an integration test that
  reads the row back from SQLite.
- `GET /metrics` returns every counter / gauge in SCOPE.md §10.2,
  verified by a curl + grep in the Goal 6 session note Outcome.
- The admin UI loads from a clean gateway checkout and shows live
  data without dev-server-only routing.
- Cross-org isolation is verified by a multi-org integration test;
  the test is enumerated in the Goal 7 session note plan table.
- The demos in SCOPE.md §13's Phase 3 and Phase 4 entries are
  reproducible from a clean checkout.
