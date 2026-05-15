# SESSION-PEER-REVIEW-IMPROVEMENTS

Design notes for evolving Codeless's session-handover model and adding
a peer-review gate between sessions. Not a spec — a proposal to argue
with before anything lands in `SCOPE.md` or `JOB-MODEL.md`.

Related:
- [`JOB-MODEL.md`](./JOB-MODEL.md) — current handover contract
- [`JOB-LOOP.md`](./JOB-LOOP.md) — tick procedure
- [`codeless-types/src/handover.rs`](../codeless/crates/codeless-types/src/handover.rs) — the wire type
- [`codeless-runtime/src/handover.rs`](../codeless/crates/codeless-runtime/src/handover.rs) — write/read/extract
- [`codeless-runtime/src/template_runner.rs`](../codeless/crates/codeless-runtime/src/template_runner.rs) — multi-stage execution
- [`AGENT.md`](./AGENT.md) — persona / subagent / runner split; per-stage persona override is the natural companion to the per-stage handover and reviewer-as-separate-session ideas below

## What the system does today (one paragraph)

A session ends; `ClaudeRunnerAdapter` looks at the assistant's final
text, extracts a fenced ` ```handover ` block, and writes it to
`<worktree>/runs/<job_id>/handover.md` with four canonical sections
(`Done` / `Next` / `What you need to know` / `Open questions`). When
the next job is dequeued, `job_driver_loop` calls
`find_latest_handover` (newest-by-mtime) and prepends
`prompt_prefix_for(...)` to the new job's prompt. Inside a single
multi-stage job, `TemplateRunner` does **not** write or read handovers
between stages — state passes via the git worktree and (optionally) a
resumed claude session id. There is no machine-checked summary, no
review gate, no retry policy.

## Does a session already leave a summary?

Yes, but with three real limitations:

1. **One handover per job, not per stage.** A 9-stage job writes one
   `handover.md` at the end of stage 9. If stage 4 of 9 was the
   important inflection point, that decision lives in the model's
   in-context memory or the commit message — not in a structured doc.
2. **Self-reported.** The model that did the work writes its own
   report card. There is no second pair of eyes between "I think I'm
   done" and "the next session believes me."
3. **Mtime-keyed discovery.** `find_latest_handover` walks
   `.codeless/worktrees/job-*/runs/*/handover.md` and picks the newest
   file by mtime. Concurrent jobs in the same repo can race; the
   "wrong" job's handover can prime the wrong next session.

## 10x ideas — handover

Ordered by leverage, not by effort.

### H1. Per-stage handover, not just per-job

Write a `Handover` at the end of every stage to
`runs/<job_id>/<stage_id>/handover.md`. The next stage's
`stage_prompt` prepends the prior stage's handover the same way
`job_driver_loop` prepends the prior job's. Hook point is the loop tail
in `TemplateRunner::run` right after `RunnerOutcome::Completed`. This
turns the four-section contract into a real cross-stage protocol
instead of a job-terminator artefact. Cost: handover discovery has to
change from "newest mtime" to "named by `(job_id, stage_id)` lookup".

### H2. Structured handover beats prose handover

Today `Handover` is `Vec<String>` per section — the model writes free
text and the runtime parses bullet lines. Push more structure where it
exists:

- `decisions: Vec<Decision { what, why, alternative_considered }>`
- `invariants: Vec<Invariant { statement, enforced_by }>`
- `followups: Vec<Followup { description, owner_stage, blocking: bool }>`
- `artefacts: Vec<Artefact { path, kind, sha }>` — every file the
  stage touched, content-hashed so the next session can detect drift

`done` / `next` / `what_you_need_to_know` stay as free prose for the
soft stuff; the structured fields make the handover queryable,
diff-able, and trustable. The UI's Handover panel becomes a real
artefact view, not a markdown render.

### H3. Handover discovery is keyed, not heuristic

Replace mtime ranking with explicit lookup: the next session asks for
`(job_id, stage_id - 1)` or `(repo_id, prior_job_id)`. Mtime survives
as a tiebreaker for "find the most recent handover in this repo I have
no other signal about", but the happy path never uses it.

### H4. Handover is published as an event, not just a file

`Event::HandoverWritten { job_id, stage_id, path, summary_sha }` flows
through the same event bus the StageRecorder consumes. The UI can
subscribe and badge the stage. The peer-review gate (below)
subscribes too instead of polling the filesystem. The file remains
the source of truth; the event is the notification.

### H5. Round-trip the handover through the model's *next* turn

When a new session starts with a prepended handover, ask it to
**acknowledge** the four sections in its first reply ("I understand
that X is done, my next step is Y, the gotchas are Z, I am resolving
the open questions before implementing: ..."). The runtime parses
the ack and stores it. If the ack disagrees materially with the
handover (NLI-style check against `done` / `next` / `open_questions`),
flag it. This is cheap and catches the "model ignored the prefix and
re-did stage 4" failure mode early.

### H6. Versioned handover schema

Stamp `schema_version: u32` on the wire type now while it is cheap. A
session that reads a newer-than-supported handover should fail closed
instead of silently dropping unknown fields, and the system prompt
hint can vary by version.

### H7. Handover validation at write time

Right now a blank `done` is just discouraged in JOB-MODEL.md prose.
Promote the rule into `Handover::validate()`:

- `done` non-empty unless `status == Aborted`
- `next` non-empty unless `status == Completed` AND this was the last
  stage of a finite plan
- `what_you_need_to_know` non-empty if the stage made any
  non-trivial-shaped commit (heuristic: more than N lines changed)
- `open_questions` empty implies *settled* — the next session reads it
  as "do not re-litigate"

`extract_handover` returns `Result<Handover, ValidationError>` and the
fallback path is reserved for genuine parse failures, not "the model
emitted a blank section".

### H8. Handover-as-prompt-budget

A handover that grows unboundedly across many stages is the slow
version of "session prompt overflows". Today `prompt_prefix_for` just
concatenates. Add a budget per session run: each handover gets a hard
ceiling (e.g. 8k tokens); compression happens at write time, not read
time, so the artefact on disk is the same one the next session sees.

## 10x ideas — peer review between sessions

The user's framing: a session, before "moving on", invokes a peer
review. The reviewer decides whether the session can hand off cleanly,
or whether a redo is required. One redo, then a fresh peer review. If
that fails too, the session escalates rather than spinning.

This maps onto existing pieces: REVIEW stages already exist in
`TemplateRunner` (they emit `Event::ReviewRequested` but do not block
today). The peer-review proposal turns the gate from advisory into
load-bearing.

### P1. Reviewer is a separate session, not the same model thread

The session that did the work is not the session that reviews it. A
peer reviewer:

- Starts fresh (no inherited claude session id).
- Sees only the handover, the diff, and the job goal / stage title.
- Has no write tools — only read + a structured `review.decide` RPC.

This is what makes it a *review* rather than a self-confirmation. If
the reviewer can edit, the gate is theatre.

### P2. Pass / fail is a typed verdict, not prose

```rust
pub enum ReviewVerdict {
    Pass { rationale: String },
    Fail { reasons: Vec<ReviewIssue>, must_address: Vec<StageId> },
    PassWithFollowup { rationale: String, followups: Vec<Followup> },
}

pub struct ReviewIssue {
    severity: Severity,        // Blocker | Major | Minor
    section: HandoverSection,  // which part of the contract failed
    quote: String,             // the specific line the reviewer is challenging
    suggested_fix: String,
}
```

`Fail` is only legal if at least one issue has `Blocker`.
`PassWithFollowup` is the escape valve for "this is fine to merge, but
note these for the next stage" — without it, reviewers will reject
fine work because they have nowhere else to put the note.

### P3. Retry policy is in the runtime, not the model

```
work-session → peer-review
  Pass                 → advance to next stage / job
  PassWithFollowup     → advance, attach followups to next session's prompt
  Fail                 → redo-session (max 1) → peer-review
    Pass               → advance
    PassWithFollowup   → advance with followups
    Fail               → escalate (human, or a different-runner session)
```

Loop bound is fixed: one redo, period. A second `Fail` does not retry
again — it surfaces an `Event::ReviewEscalated` and parks the job in
`AwaitingHumanReview`. The point of bounding the loop is to make the
failure mode *visible* instead of *expensive*.

The retry count lives on the stage row (`stages.review_attempts`),
not in the model's prompt — the runtime enforces it. The redo session
sees a new handover-shaped artefact: the original handover + the
reviewer's `Fail` verdict + the must-address list.

### P4. Handover-for-redo is its own shape

A redo session's input is not just "the prior handover" — it is a
three-part packet:

1. The original handover from the failed work-session.
2. The reviewer's structured verdict (issues, quotes, suggestions).
3. A constraint section: "you have one attempt; address these
   specific blockers; do not re-architect."

Schema-wise this is a new wire type, not a `Handover`. Call it
`RedoBriefing` so the model and the runtime can tell them apart.

### P5. Reviewer selection is a policy hook

MVP: same runner, fresh session. Phase-2: different runner (claude
work-session → codex peer-review) so a single-model failure mode
cannot pass itself. Phase-3: deterministic reviewer for cheap checks
(lint / typecheck / test run as a "reviewer" with its own typed
verdict) before the LLM reviewer ever runs. The interface stays
`Reviewer -> ReviewVerdict`; what produces the verdict is policy.

### P6. The review is an event-stream artefact

Every review writes:

- `runs/<job_id>/<stage_id>/review-<n>.md` (n = attempt number)
- `Event::ReviewStarted { review_id, stage_id, reviewer_kind }`
- `Event::ReviewDecided { review_id, verdict_kind, issue_count }`

The UI's Timeline interleaves these with the existing stage events.
A user reading the job page sees: stage started → stage completed →
review started → review failed → redo started → redo completed →
review started → review passed → next stage.

### P7. The review can challenge the handover itself

A common reviewer failure mode if you only show it the diff: it
reviews the code but never notices the handover claims something that
did not happen. Make the handover a first-class input: the reviewer's
prompt explicitly asks "does the handover's `done` list match the
diff? does `what you need to know` capture the real gotchas?" An
inaccurate handover is itself a `Fail` reason (`section:
HandoverSection::Done`).

### P8. Cost / time caps on the review loop

A review session has its own budget envelope. The default should be
small — a review that takes longer than the work it reviewed is a
smell. The retry loop has a job-level cap too: if total review-spend
exceeds N% of total work-spend, escalate without running another
review. This protects against pathological "two models argue forever"
runs.

### P9. The review verdict feeds the next handover's prompt prefix

When the next stage starts, its prefix is not just the prior
handover; it is `handover + last_review_verdict`. The next stage sees
both "what the prior session claims it did" and "what the reviewer
agreed was actually done." When those disagree, the next session has
the signal to investigate before building on top.

### P10. Human override is always a verdict

A human reading the job page can click `Override -> Pass` or
`Override -> Fail` and the runtime treats it as a `ReviewVerdict` with
`reviewer_kind: Human`. Same wire shape, same event, same retry
accounting. The point is to not have two parallel state machines (one
for AI review, one for human review) — there is one gate, multiple
kinds of reviewer.

## What this implies for SCOPE.md

If we adopt any meaningful subset of the above, four things need to
move:

1. `stages` table grows: `review_attempts INT`,
   `last_review_verdict JSON`, `archived` stays as-is.
2. `Handover` wire type gets `schema_version`, `decisions`,
   `invariants`, `followups`, `artefacts` — and a `validate()` method
   the runtime calls before writing.
3. A new `Reviewer` trait alongside the `Runner` trait, with the same
   "runner factory" shape (a `ReviewerFactory` that produces a
   `Reviewer` per stage based on policy).
4. REVIEW stages in the template runner become real blocking gates,
   not advisory events. The `Event::ReviewRequested` emission stays;
   what changes is the runtime waits on the resulting verdict before
   advancing.

R1 (crate dependency direction) is unaffected — reviewers live in
`codeless-runtime` and `codeless-adapters-host` like the existing
runners. R2 (single transport) is unaffected — the verdict is a wire
type, the UI consumes it via `RpcClient`. R4 (SQLite source of truth)
gets reinforced: the verdict lands in the `reviews` table, not in a
sidecar JSON.

## Minimum-viable first slice

If we want a 10x change but not a 10-month project, ship this
sequence:

1. **Per-stage handover** (H1) — pure additive change in
   `TemplateRunner`; no schema migration.
2. **Handover validation at write time** (H7) — pure code change in
   `codeless-types`; rejects blank-done handovers.
3. **REVIEW stages actually block** — flip the existing
   `Event::ReviewRequested` from advisory to load-bearing using the
   review row that already exists in the schema.
4. **Typed verdict** (P2) — the smallest version: `Pass | Fail`, no
   followups, no retry, just a gate.
5. **One redo, then escalate** (P3) — fixed-bound retry on `Fail`.
6. **Different-runner reviewer** (P5 phase-2) — once 1-5 are stable.

Everything else (structured handover, NLI ack check, deterministic
reviewers, cost caps) is a refinement on top of that backbone.

## Open questions

- Does the reviewer see the *prompts* the worker saw, or only the
  *outputs*? Seeing prompts catches "the worker ignored a constraint";
  not seeing prompts keeps the review honest about the artefact.
  Probably: see both, in separate clearly-labelled sections.
- When a redo runs, does it inherit the worker's claude session id
  (cheap, continuity) or start fresh (clean slate, no rationalising)?
  Probably: fresh — the point of a redo is to not double down.
- Is there a "PassWithFollowup that the next stage must address"
  versus "PassWithFollowup that anyone can pick up later"? Probably
  yes; the runtime should know the difference because it drives whether
  the next stage's prompt prefix includes the followups as blockers.
- Does the human override count against the retry budget? Probably
  no — a human verdict resets the counter; the budget exists to bound
  *AI* loops, not human ones.
