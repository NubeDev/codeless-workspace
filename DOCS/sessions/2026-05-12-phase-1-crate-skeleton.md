# Build status — Phase 1 crate skeleton

> ⛔ **AGENT REMINDER — READ BEFORE TOUCHING THIS FILE**
>
> 1. You are running JOB-LOOP. Spec: `DOCS/JOB-LOOP.md`. Project scope:
>    `DOCS/SCOPE.md`. Code-style rules: `CLAUDE.md` (repo root).
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
>    optional and not "later". A tick that ends with unpushed commits
>    means the next tick (or the next agent, after `/clear` or a fresh
>    session) sees stale remote state and can clobber or duplicate work.
>    `./bin/mani --config mani.yaml run commit --projects codeless` then `mani run push --projects
>    codeless` — both, every tick, no exceptions. If push fails, mark
>    the stage `[!]` and halt. Never `--force`, never `--no-verify`.
> 6. ⛔ **CODE COMMENTS ARE LOAD-BEARING — WRITE THEM CAREFULLY.**
>    Comments are how the *next* AI agent (and the next human) understands
>    intent. Rules:
>    - Explain **why**, not what. The code already says what.
>    - **No emojis.** Anywhere. Ever.
>    - **No task-status comments.** Never reference stages, ticks,
>      milestones, "added in stage 3", "TODO from M5", "fixed for ticket
>      X". Comments describe the code as it stands, not the task that
>      produced it.
>    - **Long-term framing.** Write for someone reading this in 6 months
>      with zero context — invariants, constraints, why this approach
>      over the obvious one.
>    - **Normal length.** A short line where one helps. A short paragraph
>      where the *why* is genuinely subtle. No multi-paragraph essays,
>      no decorative banners, no ASCII art.
> 7. ⛔ **CROSS-PLATFORM REACH IS ENFORCEABLE.** Stages that touch Rust
>    crates respect the iOS-safe / Android-safe columns in
>    `DOCS/SCOPE.md` "Crate layout". Stages that touch UI modules import
>    only `RpcClient` — never `@tauri-apps/api/core` directly. Trip
>    either rule → mark stage `[!]` and halt.

File: DOCS/sessions/2026-05-12-phase-1-crate-skeleton.md
Goal: Land Phase 1 — types, in-process RPC, specta codegen, initial sqlx
      migration, runtime state machine with MockRunner, tracing baseline,
      codeless/CLAUDE.md, secrets CLI, worktree manager, and a green
      end-to-end `codeless run --once` against a runner — all pushed.
Started: 2026-05-12
Last tick: 2026-05-12 (tick 1 — stage 1)
Current stage: 2 / 11

Repo:        codeless
Branch:      feat/bootstrap-cargo-workspace
Memory policy: compact every 3 stages
Scheduler:   CronCreate one-shot, ~1 min between ticks
Max ticks:   30

## Stages
Format: `[ ] N. [S|M|L] title` — complexity tag is mandatory.
`L` stages must be split into S/M sub-stages before being worked.

- [x] 1. [S] codeless-types: Repo/Job/Stage/Task/Event/Review structs (serde)
- [ ] 2. [M] codeless-rpc trait + in-process implementation  ← next
- [ ] 3. [S] specta wire-type generation + snapshot test
- [ ] 4. [S] sqlx initial migration matching SCOPE.md Appendix A
- [ ] 5. [M] codeless-runtime state-machine skeleton + MockRunner test harness
- [ ] 6. [S] tracing-subscriber JSON-to-stdout baseline
- [ ] 7. [S] codeless/CLAUDE.md at repo root capturing the rules from SCOPE.md
- [ ] 8. [S] codeless secrets set/get/rm/list against chmod 600 secrets.toml
- [ ] 9. [S] Worktree manager: git worktree add/remove + reaper-on-startup
- [ ] 10. [M] codeless run --once --repo <r> "<prompt>" end-to-end against
         a chosen runner, streaming events to stdout
- [ ] 11. [S] Phase 1 wrap-up: README pointer, CODELESS.md memory update,
         confirm cargo test --workspace + clippy -D warnings + fmt --check green

Likely batching (planning hint, not a contract):
- Tick 1: stage 1 (S) — types only, isolated, low risk; gives later stages something to import.
- Tick 2: stage 2 (M) — RPC trait + in-process impl.
- Tick 3: stages 3 + 4 (2×S, both wire/schema-adjacent).
- Tick 4: stage 5 (M) — runtime + MockRunner.
- Tick 5: stages 6 + 7 (2×S, mechanical).
- Tick 6: stages 8 + 9 (2×S, both adapters-host adjacent).
- Tick 7: stage 10 (M).
- Tick 8: stage 11 (S) — wrap-up.

Note: bootstrap stage "Cargo workspace + crate stubs" is already complete
(landed in the bootstrap loop). Workspace stub branch is the base for
Phase 1 — see commit ebd18a5.

## Notes
- Working branch is feat/bootstrap-cargo-workspace (the existing branch
  that already carries the 8 crate stubs). User confirmed reuse rather
  than cutting a new feat/phase-1-skeleton.
- codeless/CLAUDE.md (stage 7) is distinct from the workspace
  CLAUDE.md at codeless-workspace root — the workspace one already
  exists from the bootstrap loop. SCOPE.md Phase 1 wants one at the
  inner-repo root too.
- Stage 1: IDs are ULID newtypes (`RepoId`, `JobId`, `StageId`, `TaskId`,
  `ReviewId`) generated by a small macro to keep the per-type boilerplate
  one line. Money is `CostCents(i64)`, time is `UnixMillis(i64)` — both
  newtype wrappers so the runtime can't accidentally hand a raw `i64` to
  the wrong column. `Event` is `#[serde(tag = "type", rename_all =
  "kebab-case")]` so every variant serializes to the wire labels listed
  verbatim in SCOPE.md "What each level means". `TaskEnqueued` carries
  `depends_on` from day one per SCOPE.md Rule 4. Crate deps: `serde` +
  `ulid` only — must remain iOS/Android-safe.

## Blockers
(none)
