# LOOP-CODER — the long-running coder loop, codeless-style

> **Sister docs.** This is the loop's *design intent and product
> constraint* doc. [`JOB-LOOP.md`](./JOB-LOOP.md) is the operational
> spec each tick follows (status file format, batching, scheduling,
> guardrails). [`SCOPE.md`](./SCOPE.md) is the product scope this
> loop sits inside (Repo → Job → Stage → Task → Review,
> single-tenant trust boundary, SQLite source of truth).
> If anything below contradicts SCOPE.md, **SCOPE.md wins** — fix
> this file rather than diverge.

## The one constraint that drives this doc

**A developer must be able to start a coder loop and walk away for
24 hours.** They wake up to a branch with shippable commits, a clear
status file showing what landed and what halted, and a small number
of `[?]` review gates waiting at the points where the loop decided
it needed a human.

That constraint is the difference between codeless's coder loop and
every other tool in the same space. It's not "agent that codes"; it's
"agent that codes *for as long as it takes*, without melting under
its own context, without burning the budget, without silently
clobbering work, and without needing a human to babysit between
ticks".

Everything else in this doc serves that constraint.

## Why "fresh session per tick" is load-bearing

Every coding agent eventually runs into the same wall: **context grows
faster than usefulness**. An LLM session that lasts an hour of real
work has accumulated tool-call results, file dumps, intermediate
reasoning, and dead ends. By hour two it's slower, more expensive,
and worse at the task than a fresh session pointed at the current
state of the repo would be.

The rubix coder loop (see [§ Comparison](#comparison-with-the-rubix-coder-loop))
runs one long-lived in-process orchestrator with a budget enforcer.
That works for sessions measured in hours. It cannot work for
sessions measured in days — the orchestrator's own conversational
context becomes the bottleneck long before any cost ceiling trips.

**Codeless solves this by treating each tick as a disposable
session.** A tick wakes up, reads the status file from disk, decides
its batch from the next pending stages, does the work, commits and
pushes, updates the status file, schedules the next tick, and exits.
The next tick is a *fresh* agent — no in-memory state, no prior
conversation, no accumulated tool output. It rebuilds its
understanding from:

- the status file (what's done, what's next, what's blocked),
- the git history on the remote (what the previous tick actually
  changed),
- `CLAUDE.md` / `CODELESS.md` (durable per-repo rules and memory),
- the codebase itself.

That's it. Everything else is disposable. The session that did
stages 1–4 has no privileged view of stages 5–9 that a fresh session
starting at stage 5 wouldn't have.

This design choice has three direct consequences:

1. **Context never grows unbounded.** Each tick starts with an empty
   conversation. Twenty-four hours of work is twenty-something fresh
   sessions, not one accumulating monster.
2. **The status file is the contract, not a convenience.** If a tick
   writes something to the status file that the next tick can't
   parse, the loop is broken. If a tick *doesn't* write something the
   next tick needs, the loop is broken. The file format is
   load-bearing.
3. **The push step is non-negotiable.** A tick that commits but
   doesn't push leaves the next tick (a different agent) reading a
   stale remote. The next tick will plan against state it can see
   and risk clobbering work it can't.

These three are the spine of [`JOB-LOOP.md`](./JOB-LOOP.md). Read
that doc for the operational rules; this doc explains *why* those
rules exist.

## What unsupervised 24-hour operation actually requires

Listing them so future contributors know which features earn their
keep against this constraint and which are nice-to-haves.

### Required (a 24-hour run is impossible without these)

- **Fresh session per tick.** Above.
- **Durable status file + remote git as the only carriers between
  ticks.** Anything in-memory is gone by morning.
- **Idempotent commit + push.** A tick that crashes between commit
  and push must be re-runnable without duplicating work or
  conflicting. Mani's `commit`/`push` tasks are idempotent (commit
  with no diff is a no-op; push with no new commits is a no-op).
- **Crash-resumption invariant.** Exactly one step in the per-stage
  ordering is the *commit point* — the moment after which the work
  is considered durable. In codeless that step is `update_status`:
  the status file lands in the same commit as the code change, and
  push follows. A crash anywhere before `update_status` replays the
  whole stage; a crash after replays only push, which is idempotent.
  Reordering the steps breaks this.
- **Push every tick, before exit.** Not "at the end of the run".
  Every tick. See JOB-LOOP § "Push every tick — why it's non-negotiable".
- **Self-scheduling successor or explicit DONE.** A tick that
  finishes work and silently exits has broken the loop. Either it
  schedules the next tick (`CronCreate` / equivalent) or it reports
  `DONE` because every stage is `[x]`, or it halts and tells the
  operator why it couldn't.
- **Per-job worktree.** SCOPE.md "Workspace = one `git worktree` per
  job" means a crashed or runaway tick can't damage the user's main
  checkout. Reaping is user-driven (`gc_worktrees`) so the user can
  always inspect what a halted tick left behind.

### Required for *useful* 24-hour operation

- **Budget ceilings on the loop itself.** Not just per-job cost
  caps — the loop needs a soft (`Escalate`) and hard (`Halt`)
  ceiling on cumulative cost, wall time, and token usage across the
  whole session. Without this, one runaway stage burns the night's
  budget on a stuck `claude` instance and the operator wakes up to
  "out of credit, no progress".
- **`[?]` approval state.** Stages that the loop *knows* it
  shouldn't auto-land (publishing a release, dropping a database
  column, anything that hits production) get an `[?]` marker. The
  loop halts on `[?]` and waits for a human. This is different from
  `[!]` (halted on failure) — `[?]` is "I did the work, I'm not going
  to publish without you".
- **Coherent halt-on-failure.** A failed verify must mark the stage
  `[!]` and stop the loop. No retry-on-failure, no "skip this stage
  and try the next" — silent skips are how a 24-hour run produces
  six green stages and three quietly broken ones.
- **Sized stages.** `S` (small, ≤ 15 min, mechanical), `M` (medium,
  one focused area), `L` (must be pre-split). Without sizes the
  batcher can't decide whether to combine; without combining, every
  S becomes its own tick and the operator wakes up to a status file
  with 80 trivial commits.

### Nice but not load-bearing

- A pretty UI showing what each tick did. The status file + git log
  cover the audit trail; the UI is a render.
- Real-time notification on halt. Convenient, not load-bearing — the
  status file's `[!]` block is the source of truth.
- Multi-repo loops in a single session. Today the loop is one
  session per (repo, branch); fan-out across repos is a future
  feature, not a 24-hour-unsupervised requirement.

## Comparison with the rubix coder loop

Rubix ships `crates/domain-coder` (in `rubix-agent`) — same JOB-LOOP
contract codeless follows, implemented as a long-lived in-process
orchestrator with a `StepDriver` trait and a `BudgetEnforcer`. See
`rubix-agent/docs/sessions/ai/CODER-LOOP.md` for the spec and
`rubix-agent/crates/domain-coder/src/` for the implementation.

The two loops share the same DNA: markdown status doc, `S/M/L` tags,
contiguous-S / single-M / L-split batching, plan → implement →
verify → update-status → commit/push ordering.

| Dimension | Rubix coder loop | Codeless coder loop |
|---|---|---|
| Session model | One long-lived in-process orchestrator. | Fresh agent per tick. |
| Bounded session length | Limited by orchestrator context + budget. | Effectively unbounded — context resets every tick. |
| Tick boundary | A `LoopDriver::run_tick` call. | An agent invocation (cron-scheduled or self-scheduled). |
| Source of truth between ticks | Markdown status doc (read once at session start, written in-process). | Markdown status doc *on disk, pushed to remote*. Every tick re-reads. |
| Crash recovery | Resume from in-memory state if the process survived, else re-read the doc. | Always re-read the doc + remote git. There is no in-memory state to lose. |
| Concurrency | One session at a time, locked on `(repo, branch)`. | Many concurrent jobs across many repos (SCOPE.md). |
| Budget enforcement | `BudgetEnforcer` with soft + hard ceilings. | Per-job cost cap; loop-level ceilings are an open item (see § Borrowed ideas). |
| Approval gate | `[?]` state in the status doc. | `Review` row in SQLite; `[?]` not yet wired. |
| Step driver | `StepDriver` trait, injected. | Inline shell-outs to mani + git inside each tick. |

### Where rubix is stronger

1. **Executable loop.** The loop rules are a Rust crate
   (`MarkdownStatusDoc` parser, `BatchPlanner`, `LoopDriver`),
   unit-testable, deterministic without an LLM in the interpretation
   path. Codeless's loop is prose in `JOB-LOOP.md` that an LLM has to
   re-interpret every tick.
2. **`StepDriver` trait.** Side effects (cargo, git, mani) are
   injected, so orchestration is testable and a new driver (e.g. for
   a non-git VCS) is a new file rather than a rewrite.
3. **Named crash-resumption invariant.** Rubix explicitly names
   `update_status` as the commit point; codeless's JOB-LOOP describes
   the order without naming the invariant.
4. **Budget ceilings.** Two-tier (escalate, halt) on tokens, cost,
   and wall time. Codeless's caps are per-job, not per-session.
5. **`verify.sh` contract.** Typed cwd, env allowlist, timeout,
   signals, stdout truncation, discovery order. Codeless's verify
   today is implicit (`cargo test && cargo clippy && cargo fmt &&
   pnpm tsc`) and only works because codeless drives its own repo.
6. **`[?]` approval glyph.** Four-state checklist instead of three.
   Plays nicely with codeless's `Review` table; the wiring is
   straightforward.

### Where codeless is stronger

1. **The 24-hour run is the design centre.** Rubix's loop assumes
   the orchestrator process stays alive across the whole session;
   codeless's loop assumes it does not. The latter is the only way
   to run for days without context collapse.
2. **Multi-job, multi-repo concurrency from day one.** Rubix's
   coder loop is one-session-at-a-time; codeless is many-jobs-many-repos
   with a scheduler and per-repo caps.
3. **SQLite as source of truth for *runs*.** Rubix's status doc is
   the only persistent state. Codeless persists every run, every
   event, every review to SQLite; the status file is for the loop's
   self-driving, the database is for the product surface.
4. **Typed wire event stream.** SSE + specta-generated TS types mean
   every UI (and every external script via the future MCP surface)
   sees the same event vocabulary in real time. Rubix's coder loop
   has a status doc, not an event stream.
5. **Single UI for four shells.** Browser, Tauri desktop, iOS,
   Android, all driven by the same `RpcClient`. Rubix's coder loop
   doesn't address shells.

### Ideas we should borrow

In priority order, lowest cost first:

1. **Name the crash-resumption invariant in JOB-LOOP.md.** One
   paragraph: "`update_status` is the commit point; push is
   idempotent so a crash after `update_status` replays only push".
   Stops a future contributor reordering the steps.
2. **`[?]` approval glyph.** Add the fourth state. A stage marked
   `[?]` opens a `Review` row in SQLite and halts the loop until the
   review resolves. Cheap and high-value.
3. **Soft + hard budget ceilings on the *session*, not just the
   job.** Reuse the existing `cost_cap_cents` / `wall_clock_cap_ms`
   shape; add an `escalate_at` ratio and a per-loop accumulator.
   Three verdicts: `Continue`, `Escalate`, `Halt`. The driver
   checks the verdict before each tick.
4. **Port the loop into a `codeless-loop` crate.** Status-doc
   parser, batch planner, `StepDriver` trait — transport-agnostic,
   unit-testable. Codeless then ships a real `/rpc/loop_*` surface
   and the LLM stops being the interpretation layer for "what stage
   is next". Largest change; largest payoff. **This does not violate
   the fresh-session-per-tick rule** — the crate is the *parser and
   planner*, not the orchestrator. Each tick still constructs it
   fresh, runs one batch, exits.
5. **`verify.sh` contract** if and when codeless drives a user's
   repo, not just its own. Until then the implicit
   `cargo + clippy + fmt + tsc` is fine.

What we deliberately do *not* borrow:

- The long-lived in-process orchestrator. That's the very thing the
  fresh-session-per-tick model rejects.
- The "profile vs framework" split (`domain-coder` as a profile of a
  generic `domain-job-loop`). Codeless's product is the coder loop;
  there is no second profile to factor against.
- MCP `coder.*` aliases as a separate surface. Codeless's MCP
  surface (SCOPE.md "MCP surface") already mirrors the RPC trait;
  loop control lives on the same RPC trait, no aliases needed.
- Skill packaging (`com.<org>.coder.<task>` extension dirs). Codeless
  jobs are YAML/TOML templates per SCOPE.md; that's the right shape
  for the product.

## Open items

- **Loop-level budget.** Today each job has its own cap; an unsupervised
  loop running 30 stages over 24 hours has no aggregate ceiling. Borrow
  rubix's two-tier model.
- **`[?]` glyph wiring.** Decide where it sits relative to the existing
  `Review` table: is `[?]` a status-doc-only marker that the loop
  respects, or does it materialise a `Review` row that the UI surfaces
  alongside in-job reviews? Suggestion: the latter — one review queue
  for the operator, no parallel concept.
- **Self-scheduling primitive.** `CronCreate` has proved unreliable
  across harnesses (see `DOCS/sessions/2026-05-12-ux-grind.md` tick
  log: durable flag ignored, one-shot crons queued but never fired a
  fresh session). Investigate `ScheduleWakeup` / `/loop` skill / a
  dedicated codeless-side scheduler that pokes the agent runner on a
  real cron. **Until this is solved, "unsupervised 24-hour run" is
  aspirational** — every tick today still needs a human to say "next".
- **`codeless-loop` crate extraction.** The biggest borrow from rubix.
  Defer until at least one more loop-driving consumer exists (e.g.
  the CLI's `codeless loop run` subcommand) so the trait shape is
  driven by two callers, not one.
