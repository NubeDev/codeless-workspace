# Build status — Phase 2a persistence + queue

> ⛔ **AGENT REMINDER — READ BEFORE TOUCHING THIS FILE**
>
> 1. You are running JOB-LOOP. Spec: `DOCS/JOB-LOOP.md`. Project scope:
>    `DOCS/SCOPE.md`. Code-style rules: workspace `CLAUDE.md` and
>    `codeless/CLAUDE.md`.
> 2. **One logical batch per tick.** Read each stage's `[S|M|L]` tag and
>    batch per JOB-LOOP.md "Hard rules" #3: up to 4 contiguous S in one
>    area, OR 1 M (+ optional related S), OR the next sub-stage of an L.
>    Verify + commit + push **each stage** via mani before moving to the
>    next stage in the batch.
> 3. **You MUST schedule the next tick before exiting** — call
>    `CronCreate` with `recurring: false` for a single fire ~1 min from
>    now. If all stages are `[x]`, report `DONE` instead. If you cannot
>    schedule, **do NOT exit silently** — tell the user which stage
>    finished, exactly why scheduling failed, and how to re-kick. See
>    JOB-LOOP.md "If you cannot schedule".
> 4. Update this file in the **same commit** as the code change.
> 5. ⛔ **COMMIT _AND_ PUSH BEFORE THE TICK ENDS.** Pushing is not
>    optional and not "later".
>    `./bin/mani --config mani.yaml run commit --projects codeless` then
>    `mani run push --projects codeless` — both, every tick, no
>    exceptions. If push fails, mark the stage `[!]` and halt. Never
>    `--force`, never `--no-verify`.
> 6. ⛔ **CODE COMMENTS ARE LOAD-BEARING — WRITE THEM CAREFULLY.**
>    Explain **why**, not what. No emojis. No task-status comments. Long-term
>    framing. Normal length.
> 7. ⛔ **CROSS-PLATFORM REACH IS ENFORCEABLE.** Stages that touch Rust
>    crates respect the iOS-safe / Android-safe columns in
>    `DOCS/SCOPE.md` "Crate layout". Trip → mark stage `[!]` and halt.

File: DOCS/sessions/2026-05-12-phase-2a-persistence.md
Goal: Replace `MemoryStore` with SQLite-backed persistence; persist
      events with cursor-replay; add a lease-based task queue with
      concurrency caps (global, per-repo, per-runner) so the
      scheduler in Phase 2b has a real queue to drive; prove
      resumability across a simulated core restart.
Started: 2026-05-12
Last tick: 2026-05-12 (tick 3 — stage 3)
Current stage: 4 / 9

Repo:        codeless
Branch:      feat/phase-2a-persistence
Memory policy: compact every 3 stages
Scheduler:   CronCreate one-shot, ~1 min between ticks
Max ticks:   30

## Stages
Format: `[ ] N. [S|M|L] title` — complexity tag is mandatory.

- [x] 1. [S] `InProcessRpc::with_db(pool)` plumbing; keep `new()` as a
         `:memory:` shortcut for tests. Migrations applied on construction.
- [x] 2. [M] Repo + Job persistence: `SqliteStore` replaces `MemoryStore`
         for repos and jobs (sqlx queries against the Appendix A tables).
         All existing in-process RPC tests stay green against the new store.
- [x] 3. [M] Event persistence + cursor allocation. `EventBus` writes
         to the `events` table first, then broadcasts; cursor comes
         from the autoincrement column, not an in-memory counter.
- [ ] 4. [M] `subscribe(since)` replays from SQLite from `since+1`  ← next
         upward, then attaches the live broadcast tail without gaps.
- [ ] 5. [M] Task queue: `enqueue_task`, `lease_next(runner_kind)`,
         `complete_task`, `fail_task`, `release_expired_leases`. Tests
         pin "no double-lease" and "expired lease reclaim".
- [ ] 6. [S] Concurrency caps honoured by `lease_next` (global +
         per-repo + per-runner). Config struct fed at construction.
- [ ] 7. [S] Lease heartbeat helper + a startup-time reaper for stale
         leases. Idempotent; safe to call repeatedly.
- [ ] 8. [M] Resumability: integration test that builds a runtime,
         queues a job + tasks, drops the runtime, rebuilds against
         the same DB file, and continues from where state left off.
- [ ] 9. [S] Phase 2a wrap-up: CODELESS.md + README quickstart
         refresh, three verify gates green, branch ready for PR.

Likely batching (planning hint, not a contract):
- Tick 1: stage 1 (S).
- Tick 2: stage 2 (M).
- Tick 3: stages 3 + 4 (one M, one tightly-related M — may split).
- Tick 4: stage 5 (M).
- Tick 5: stages 6 + 7 (2×S, both queue-adjacent).
- Tick 6: stage 8 (M).
- Tick 7: stage 9 (S).

## Notes
- Stage 3: `EventBus` now owns the `SqlitePool` and is fallible-async
  on publish (`sqlx::Result<EventCursor>`). The `AtomicI64` cursor
  counter is gone — cursors come from the `events.cursor` AUTOINCREMENT
  via `INSERT … RETURNING cursor`, which keeps a single allocator for
  the column and survives restarts. Persistence ordering: row INSERT
  first, then `broadcast::send` — readers either see the row first via
  the (forthcoming) since-replay path or the broadcast first via the
  live tail, never both, and the cursor monotonicity holds either
  way. The `Event::*` variant is decomposed by `split_event_json`
  (one place that knows about `#[serde(tag = "type")]`) into the
  `type` column (kebab-case label) and a `payload` JSON object that
  omits the discriminator. This isolates wire-format knowledge from
  the rest of the bus. Callers all gained `.await.map_err(db_err)?`
  on publish: `rpc.rs` (4 sites), `driver.rs` (2 sites). `MockRunner`
  maps a publish failure to `RunnerOutcome::Failed` rather than
  panic, so a DB error mid-run lands as a clean `job-failed`. New
  `tests/event_persistence.rs` pins the three contracts:
  (a) `repo-added` lands with `type='repo-added'`, payload object
  carrying `repo_id`, no `type` key leaked into payload; (b) cursors
  are 1, 2, … in publish order; (c) live subscribers still see the
  envelope with the assigned cursor after persistence.
- Stage 2: `MemoryStore` is gone; `SqliteStore` (`src/store.rs`) is now
  the sole persistence path for `Repo` and `Job`. All eight methods
  (`insert_repo`, `get_repo`, `remove_repo`, `list_repos`,
  `insert_job`, `get_job`, `update_job`, `list_jobs`) are async +
  fallible — `sqlx::Error` bubbles to callers; `InProcessRpc` maps
  it to `RpcError::Internal` via a small `db_err` helper. Enum
  encoding choice: status/stop-reason columns use explicit pattern
  matches (`job_status_label`/`parse_job_status` etc.) rather than
  serde_json round-trips on the enum value. The labels are wire-stable
  per SCOPE.md Appendix A; an explicit match makes a future drift a
  compile error rather than a silent string change. `git_auth` does
  go through serde_json because its variants carry data; column is
  TEXT NOT NULL by the migration. `parking_lot` stays (used by
  `MockRunner` for the scripted-steps cell); `serde_json` was added
  here. `rpc.store()` accessor now returns `&Arc<SqliteStore>`, so
  callers that previously did `store.get_job(id)` synchronously now
  `await` it. `driver.rs` updated accordingly. All 12 existing rpc /
  driver tests pass against the new store; clippy + fmt clean.
- Stage 1: `InProcessRpc::new()` and `InProcessRpc::with_db(pool)` are
  now async + fallible (`Result<Self, sqlx::Error>`). Both run the
  Appendix A migrator on construction so callers never have to
  remember a separate setup step. Default constructor uses an
  `sqlite::memory:` pool — sqlx pools that URL by keeping a single
  dedicated connection alive for the pool lifetime, so successive
  queries against the same `InProcessRpc::new()` see the same data
  (which is the property our tests depend on). Drop the `Default`
  impl and `with_capacity` — neither was used by any caller, and a
  fallible async constructor cannot implement `Default` anyway.
  New `pool()` accessor exposes the `SqlitePool` for the upcoming
  query-based store. Two-test pair `tests/rpc_with_db.rs` pins
  schema-after-construction + idempotent re-migration. All 12
  existing rpc/job-driver test sites got `.await.unwrap()` appended.
- Branch `feat/phase-2a-persistence` cut from
  `feat/bootstrap-cargo-workspace` at the Phase 1 wrap-up commit. PR
  target is the same parent branch (or a new `develop` once we have
  one); decided at wrap-up.
- The Appendix A migration already exists (Phase 1 stage 4); this loop
  only adds queries against it, no new migrations expected. New
  migrations land as `migrations/0002_*.sql` — forward-only.
- The `MemoryStore` is **deleted** at the end of stage 2, not kept as
  a test double. The integration tests will use a `:memory:` SQLite
  pool (which sqlx supports out of the box) — that gives us the same
  per-test isolation without a parallel implementation drifting.
- `tokio::sync::broadcast` stays as the live-event fan-out; SQLite is
  the cursor authority. The two are consistent because every publish
  goes "INSERT events RETURNING cursor → broadcast envelope" in that
  order — readers either see the row first (via the since-replay) or
  the broadcast first, never both, and the cursor monotonicity is
  preserved either way.

## Blockers
(none)
