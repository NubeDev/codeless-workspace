# Chat for this job

## user @ 2026-05-14T06:44:24Z

can you check if you undertsand the scope, and can access the docs

## assistant @ 2026-05-14T06:44:38Z

Yes — I have access to the workspace docs and CLAUDE.md rules. Quick summary of the scope as I understand it:

**Codeless** is a staged, reviewable AI coding job runner:
- Headless Rust core (`codeless-runtime`) managing many concurrent jobs across repos, each in a `git worktree`
- SQLite as source of truth for runs/events
- One React UI (`ui/codeless-ui/`, Terax-derived) shipping to four shells (browser, Tauri desktop, iOS, Android) via a single `RpcClient` interface
- Two transport impls: `HttpSseClient` (browser/mobile) and `TauriIpcClient` (desktop)
- Coding work driven by CLI wrappers (Claude Code, Codex, Copilot) or direct APIs

**Hard rules I'm operating under:**
- **R1** Process spawn only in `codeless-adapters-host`; mobile-safe crates (`-types`, `-rpc`, `-client`) never depend on host-only crates
- **R2** UI imports only `RpcClient` — no `@tauri-apps/*` or direct `fetch` to server
- **R3** One UI codebase, no per-shell `.tsx` variants
- **R4** SQLite is source of truth
- **R5** Single-tenant trust boundary in MVP
- Comments explain *why*, never *what*; no emojis, no task-status comments, no drive-by refactors
- In JOB-LOOP ticks: commit+push via `./bin/mani`, never raw git; schedule next tick before exit

Key docs I can pull as needed: `SCOPE.md`, `JOB-LOOP.md`, `MANI.md`, `UI-ARCHITECTURE.md`, `UI-PORT-AUDIT.md`, plus per-repo `CODELESS.md` and session files under `DOCS/sessions/`.

Current branch is `master` with an uncommitted edit to `ui/codeless-ui/src/app/App.tsx`. You're viewing job `01KRJJYQZJX7F5ETB6R696PXZ8`. What would you like to do?

## user @ 2026-05-14T06:47:37Z

can you check if you undertsand the scope, and can access the docs

## assistant @ 2026-05-14T06:47:45Z

Yes on both. I can read the workspace docs and I've internalized the scope.

**Scope in one line:** Codeless is a staged, reviewable AI job runner — headless Rust core (`codeless-runtime`) managing many concurrent jobs in `git worktree`s, SQLite as source of truth, one React UI (`codeless/ui/codeless-ui/`) shipping to four shells (browser, Tauri desktop, iOS, Android) through a single `RpcClient` interface.

**Hard rules I'm bound by (from CLAUDE.md):**
- **R1** Process spawn only in `codeless-adapters-host`; mobile-safe crates never reach it
- **R2** UI imports only `RpcClient` — no `@tauri-apps/*`, no direct `fetch` to server
- **R3** One UI codebase, no per-shell `.tsx` variants
- **R4** SQLite is source of truth
- **R5** Single-tenant trust boundary in MVP
- Comments explain *why* not *what*; no emojis; no drive-by refactors; no half-finished impls
- In JOB-LOOP ticks: commit+push via `./bin/mani`, never raw git; schedule next tick before exit

**Docs I can pull on demand:** `DOCS/SCOPE.md`, `JOB-LOOP.md`, `MANI.md`, `UI-ARCHITECTURE.md`, `UI-PORT-AUDIT.md`, session files under `DOCS/sessions/`, plus inner-repo `codeless/CODELESS.md`.

What do you want to work on?
