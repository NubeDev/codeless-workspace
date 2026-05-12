# CLAUDE.md — rules for agents working in codeless-workspace

You are working in `codeless-workspace`, a multi-repo workspace for the
**Codeless** project. Read this file first; it captures the rules that
make the project's cross-platform plan work. The full design is in
[`DOCS/SCOPE.md`](./DOCS/SCOPE.md). The autonomous build loop is
[`DOCS/JOB-LOOP.md`](./DOCS/JOB-LOOP.md). Multi-repo workflow is
[`DOCS/MANI.md`](./DOCS/MANI.md).

## What Codeless is (one paragraph, so you don't have to grep)

Codeless is a staged, reviewable AI coding job runner. A headless Rust
core (`codeless-runtime`) manages many concurrent jobs across many repos,
each in an isolated `git worktree`. Clients are a browser UI (MVP), a
CLI, and later Tauri desktop + iOS/Android. The same React UI ships to
all four shells through a single `RpcClient` interface; the same Rust
core powers all backends. SQLite is the source of truth for runs.
Coding work is driven by either CLI wrappers (Claude Code, Codex,
Copilot CLI) or direct APIs (Anthropic, OpenAI, OpenAI-compat).

If anything below contradicts SCOPE.md, **SCOPE.md wins** — open an
issue and update this file rather than diverging.

## Repo layout

```
codeless-workspace/        ← this repo (NubeDev/codeless-workspace, public)
├── DOCS/                  ← SCOPE, JOB-LOOP, MANI, session files
├── bin/mani               ← bundled mani binary; do not replace casually
├── mani.yaml              ← workspace task config
├── ai-runner.PATCHES.md   ← log of every codeless-side edit to ai-runner/
├── ai-runner/             ← VENDORED from rubix-agent, no .git of its own;
│                            patched in-place by this workspace — every patch
│                            lands a row in ai-runner.PATCHES.md and a
│                            `// codeless-patch-NNN` marker in source
└── codeless/              ← INNER REPO (NubeDev/codeless), independent git
                             — colocated, NOT a submodule. The workspace
                             .gitignore excludes it.
```

The inner `codeless/` repo has its own commits, branches, PRs, and
GitHub history. The workspace tracks shared tooling and docs only.

The React UI lives **inside** the inner repo at
[`codeless/ui/codeless-ui/`](./codeless/ui/codeless-ui/) — a
Terax-derived React 19 + TypeScript app that already includes editor,
terminal, file explorer, AI chat panel, settings, and themes. It is
the single UI that ships to all four shells (browser, Tauri desktop,
iOS, Android). New work converts Tauri-coupled call sites to use
`RpcClient`; the load-bearing mental model is
[`DOCS/UI-ARCHITECTURE.md`](./DOCS/UI-ARCHITECTURE.md), the
file-by-file conversion list is
[`DOCS/UI-PORT-AUDIT.md`](./DOCS/UI-PORT-AUDIT.md). Read those before
touching anything under `codeless/ui/`.

## Hard rules — violating any of these halts work

These rules are enforceable by `cargo check` or simple grep. Trip one
and the JOB-LOOP loop halts; a human must intervene.

### R1 — Crate dependency direction (Rust)

The crate table in [`DOCS/SCOPE.md`](./DOCS/SCOPE.md#crate-layout-load-bearing-not-aspirational)
defines which crates are **iOS-safe** and **Android-safe**. The mobile
shell (`codeless-tauri-mobile`, Phase 6) depends only on
`codeless-types` + `codeless-client`. **Process spawn (`tokio::process`,
`std::process::Command`) lives only in `codeless-adapters-host`** —
it must never be reachable from a mobile-safe crate's dependency graph.

This is enforced by Cargo features: process-spawning runners gate
behind a feature that the mobile build does not enable.

If you add a `use std::process` or `tokio::process` import to any crate
other than `codeless-adapters-host`, you are violating this rule.

### R2 — Single transport, single client interface (TypeScript UI)

The single React UI (`ui/codeless-ui/`, Phase 1+) imports **only**
`RpcClient`. It must **never** import:

- `@tauri-apps/api/core` — leaks Tauri to browser/mobile shells
- `@tauri-apps/api/event` — same reason
- `fetch(...)` directly to the codeless server — that's the
  `HttpSseClient` implementation's job, not the UI's

The shell decides which `RpcClient` implementation to inject:
`HttpSseClient` (browser/mobile) or `TauriIpcClient` (desktop). The UI
never knows the difference. This is what makes "one codebase, four
shells" actually work.

### R3 — One UI framework, forever

There are **no** per-shell UI files. No `Foo.web.tsx`, no
`Foo.mobile.tsx`, no `Foo.ios.tsx`. Responsive design and shell-injected
behaviour interfaces (clipboard, file picker, biometric unlock) cover
every per-platform difference. If a Tauri-mobile feature genuinely
doesn't work, write a thin **native Tauri plugin** in Rust — never a
parallel UI.

### R4 — SQLite is the source of truth

Job state, queue, events, reviews — all in SQLite. The UI subscribes
to events via `RpcClient.subscribe()`; it does not maintain its own
chat/session stores backed by anything else. If a feature wants in-memory
state on the client, it's UI presentation state only.

### R5 — Single-tenant trust boundary

Codeless MVP is **one user, one trust boundary, many concurrent jobs**.
The bearer token authorises browser, CLI, and mobile clients identically.
Do **not** introduce per-job auth scopes, per-user permissions, or
multi-tenant isolation in MVP code. Phase 7 replaces this with OIDC.

## File-level rules

### Single responsibility per file

One concept per file. If a file owns more than one of: a struct, its
methods, its associated traits, its tests — split.

### Comments — explain *why*, never *what*

The code already says what. Comments earn their keep when they capture:

- An invariant that isn't visible from the type signature.
- A constraint that explains an apparently-redundant check.
- The alternative you considered and rejected, with the reason.

Do **not** write:

- Emojis, anywhere, ever (even in TODOs).
- Task-status comments: "added in stage 3", "TODO from M5", "fixed for
  ticket X". The comment must still make sense after the loop finishes
  and the branch merges.
- Restatements: `// increment counter` above `counter += 1` is noise.
- Decorative banners, ASCII art, dividers.
- Multi-paragraph essays. If you need three paragraphs, the design is
  probably wrong — fix the code or move the explanation to `DOCS/`.

The test for a comment: would a brand-new agent reading this file
alone, with no chat history, understand *why* this code is shaped this
way? If yes, the comment is doing its job.

### No drive-by refactors

A bug fix doesn't need surrounding cleanup. A one-shot change doesn't
need a helper. Don't design for hypothetical future requirements. Three
similar lines is better than a premature abstraction.

### No half-finished implementations

If you can't complete a stage, mark it `[!]` in the session doc and
halt. Do not commit a partial implementation with a TODO.

## JOB-LOOP rules (when you're in a tick)

The full loop spec is [`DOCS/JOB-LOOP.md`](./DOCS/JOB-LOOP.md). The
load-bearing parts:

1. **You MUST schedule the next tick before exiting** (or report DONE
   if all stages are `[x]`, or follow "If you cannot schedule"). A tick
   that exits silently is a bug.
2. **Commit AND push every stage via mani** — never raw git from inside
   a tick. The mani env-var form is `KEY=value` *as a positional after
   the task name*, not a shell prefix:
   ```bash
   ./bin/mani --config mani.yaml run commit --projects codeless \
     MSG='stage N: <title>'
   ./bin/mani --config mani.yaml run push --projects codeless
   ```
3. **One logical batch per tick**, sized by complexity tags (`S` / `M` /
   `L`). See JOB-LOOP.md "Hard rules" #3.
4. **No `--force`, no `--no-verify`.** If a hook fails, fix the cause.
5. **The session file is the source of truth.** Update it in the same
   commit as the code change.

## When you're not in a tick (interactive sessions)

The same rules apply, with one relaxation: you don't have to schedule
a successor. You still have to:

- Commit + push real work, not leave it as uncommitted local changes.
- Match the comment standards.
- Not violate the cross-platform rules (R1-R5 above).
- Update relevant docs (CODELESS.md project memory, session doc if a
  loop is running).

## Pointers

- Project scope, all decisions, all open questions: [`DOCS/SCOPE.md`](./DOCS/SCOPE.md)
- Loop spec + tick procedure: [`DOCS/JOB-LOOP.md`](./DOCS/JOB-LOOP.md)
- Loop kickoff template: [`DOCS/JOB-LOOP-KICKOFF.template.md`](./DOCS/JOB-LOOP-KICKOFF.template.md)
- Multi-repo workflow: [`DOCS/MANI.md`](./DOCS/MANI.md)
- Active session docs: [`DOCS/sessions/`](./DOCS/sessions/)
- UI architecture (one codebase, four shells): [`DOCS/UI-ARCHITECTURE.md`](./DOCS/UI-ARCHITECTURE.md)
- UI conversion grind (per-file worklist): [`DOCS/UI-PORT-AUDIT.md`](./DOCS/UI-PORT-AUDIT.md)
- UI tree: [`codeless/ui/codeless-ui/`](./codeless/ui/codeless-ui/) — Terax-derived React + TS, single source for all four shells
- Project memory (per-repo): [`codeless/CODELESS.md`](./codeless/CODELESS.md)
