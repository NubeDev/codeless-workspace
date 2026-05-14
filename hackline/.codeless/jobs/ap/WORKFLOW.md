# Workflow

How to drive Goal 4 and Goal 5. The shape mirrors how Goals 0–3 ran
(see `DOCS/sessions/2026-05-14-goal3-auth-cli.md` for the gold
example).

## Before any stage starts

1. Read [`../../../SCOPE.md`](../../../SCOPE.md) end-to-end. Pay
   particular attention to §3 (architecture), §5 (wire), §7
   (persistence), §13 (phasing).
2. Read every `DOCS/sessions/2026-05-14-goal*.md` to understand what
   landed already and the conventions used (plan table, Outcome,
   Design sections).
3. Skim the existing crates so you know where things live:
   `crates/hackline-gateway/`, `crates/hackline-agent/`,
   `crates/hackline-client/`, `crates/hackline-proto/`,
   `crates/hackline-cli/`.

## Per-stage protocol (applies to Goal 4 and Goal 5)

1. **Plan the stage**. Open the session note file for the stage
   (`DOCS/sessions/2026-05-14-goal4-message-plane.md` for stage 1,
   `DOCS/sessions/2026-05-14-goal5-cmd-api-host-routing.md` for
   stage 2). Write the plan table first — every row marked `[ ]`.
   Commit the empty plan as the first commit of the stage.
2. **Implement one step at a time**, ticking `[ ]` → `[x]` in the
   session note as each step lands. Commit per step or per logical
   batch; commit messages start with `goal4: ` or `goal5: `.
3. **Verify between steps** with `cargo check --workspace`. Run
   `cargo test --workspace` after any persistence, router, or SDK
   change.
4. **Write the Outcome + Design sections** of the session note
   before closing the stage. Outcome lists what was verified (curl,
   netcat, integration tests). Design captures the why of the
   non-obvious decisions, especially anything that pushed back on
   SCOPE.md.
5. **End-of-stage gate**:
   - `cargo check --workspace` clean (no new warnings).
   - `cargo test --workspace` passes.
   - Session note has every row `[x]` plus Outcome and Design.
   - Manual demo command in the Outcome works copy-pasted.

## SCOPE.md drift

If during the stage the right design diverges from SCOPE.md, **stop
and update SCOPE.md in the same commit** that introduces the change.
Do not let the code outrun the doc; goal3-auth-cli specifically
called this rule out and it has held.

## Out-of-band rules

- Never push to `origin`. The remote is wrong for this checkout —
  commits stay local until the operator pushes.
- Never run `git rebase`, `git reset --hard`, or `git push --force`.
- Do not modify the auth layer (`crates/hackline-gateway/src/auth/`)
  beyond adding the middleware to new routes.
- New REST routes must be protected by the existing AuthedUser
  extractor unless they belong to the unauthenticated set defined
  in `DOCS/AUTH.md` (health, claim/status, claim).
- New tables go in a fresh `Vnnn__*.sql` migration file under
  `crates/hackline-gateway/migrations/`; never edit a landed
  migration.
- Ring-buffer pruning runs inside the same transaction as the
  insert, per SCOPE.md §7.

## Definition of done for the whole job

- `DOCS/sessions/2026-05-14-goal4-*.md` and
  `DOCS/sessions/2026-05-14-goal5-*.md` both exist and are complete.
- All new REST endpoints listed in SCOPE.md §5.3 for Phase 1.5 and
  Phase 2 exist, are auth-gated where appropriate, and have curl
  commands documented in their session note's Outcome.
- The demos in SCOPE.md §13's Phase 1.5 and Phase 2 entries are
  reproducible from a clean checkout.
