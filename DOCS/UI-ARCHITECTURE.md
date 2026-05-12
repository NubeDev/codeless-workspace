# UI architecture — one codebase, four shells

This is the load-bearing mental model for Phase 2. If anything in
[`SCOPE.md`](./SCOPE.md) "One UI, four shells" is unclear, this is the
working translation. If anything below contradicts SCOPE.md,
**SCOPE.md wins** — open an issue and update this file rather than
diverge.

## The goal in one sentence

The Terax UI (editor, terminal, file explorer, AI chat panel,
settings, themes) **is** the Codeless UI. It already exists, it works,
and we keep it. The only thing we change is how it talks to its
backend: every call that today goes through `@tauri-apps/*` APIs or
plugins is re-routed through a single typed `RpcClient` interface (or
a shell-injected capability adapter, for things that aren't transport
— clipboard, file picker, biometric). Once that's done, the same UI
code runs in four places — desktop (Tauri), browser, iOS, Android —
by injecting different `RpcClient` implementations at the shell
entry.

We do **not** build a separate UI. We do not build per-shell
component variants. We do not maintain a parallel app for browser /
mobile. The work is a swap of the I/O boundary, not a rewrite of the
shell.

## The reusable layer (boundary)

```
┌────────────────────────────────────────────────────────────────┐
│  Terax-derived React UI in codeless/ui/codeless-ui/src/        │
│  (editor, terminal, explorer, ai chat, settings, …)            │
│                                                                │
│  Every component imports useRpc(), useRepos(), useJob(), …     │
│  Nothing imports @tauri-apps/api/core or fetch() directly.     │
└──────────────────────────┬─────────────────────────────────────┘
                           │
                           ▼
                ┌──────────────────────┐
                │  RpcClient interface │   ← one TS interface
                │  (src/lib/rpc/)      │     every UI module imports
                └──────────┬───────────┘
                           │
            ┌──────────────┼─────────────┬───────────────┐
            ▼              ▼             ▼               ▼
       HttpSseClient  TauriIpcClient  HttpSseClient  HttpSseClient
       (browser)      (desktop)       (iOS)          (Android)
            │              │             │               │
            ▼              ▼             ▼               ▼
       codeless-      codeless-     codeless-       codeless-
       server         runtime in    server          server
       (axum)         the same      (axum)          (axum)
                      Tauri proc

shell-side adapters (clipboard, file picker, biometric)
are likewise injected, never imported by components
```

## Why this works

- **One UI tree, period.** No `Foo.web.tsx`, no `Foo.mobile.tsx`. A
  bug fix to the file explorer is a bug fix in all four shells. A
  redesign of the chat panel ships everywhere on the same commit.
- **Shell decides transport.** The browser entry constructs an
  `HttpSseClient`; desktop constructs a `TauriIpcClient`; mobile
  constructs an `HttpSseClient` against the user's chosen host. Each
  shell entry is ~30 lines.
- **Behaviour differences live behind interfaces.** Clipboard, file
  picker, share sheet, biometric unlock: each is an injected
  capability adapter. Components depend on the interface, the shell
  picks the implementation.

## What's already built

In `codeless/ui/codeless-ui/src/lib/rpc/`:

- `client.ts` — `RpcClient` interface (call + subscribe).
- `wire.ts` — wire types mirrored from `codeless-types` (will be
  replaced by direct codegen output when the Phase 1 specta step is
  wired into the UI build).
- `methods.ts` — method args/results mirrored from `codeless-rpc`
  (same caveat).
- `error.ts` — `RpcError` + HTTP status mapping.
- `http-sse-client.ts` — REST + SSE implementation for browser/mobile.
- `mock-client.ts` — in-memory implementation for dev and tests.
- `provider.tsx` — React `<RpcProvider>` + `useRpc()` hook.
- `hooks.ts` — `useRepos`, `useJobs`, `useJob`, `useEventStream`.
- `config.ts` — base URL / token resolution (localStorage → env →
  `window.origin`).

What is **not** built yet:

- `TauriIpcClient` — second `RpcClient` impl wrapping `invoke()` and
  Tauri events. Land in Phase 2 alongside the file-by-file conversion
  (see [`UI-PORT-AUDIT.md`](./UI-PORT-AUDIT.md) "Available now") so
  the desktop shell stays viable from day one rather than waiting on
  Phase 5.
- PTY (`openPty`) and blob upload (`uploadBlob`) on `RpcClient` —
  PTY needs WebSocket; blob upload needs multipart. Both are real but
  small; arrive when the corresponding terminal / upload surface is
  converted.

## The actual Phase 2 grind: convert Terax surfaces

The 31 files in [`UI-PORT-AUDIT.md`](./UI-PORT-AUDIT.md) currently
import `@tauri-apps/*`. Each one needs to be re-routed through
`useRpc()`. This is the bulk of Phase 2; nothing else moves the
product forward more.

Status by surface, with the RPC method each conversion needs and
the matching group in [`UI-PORT-AUDIT.md`](./UI-PORT-AUDIT.md) for
file-level detail:

| Terax surface | Files | RPC / mechanism | Audit group |
|---|---|---|---|
| AI chat panel | `modules/ai/lib/agent.ts` (replace), `composer.tsx`, `lib/native.ts` | `submit_job`, `subscribe`, `stop_job` | Available now |
| Settings → provider keys | `modules/ai/lib/keyring.ts`, `settings/components/ProviderKeyCard.tsx`, `settings/sections/ModelsSection.tsx` | `secrets.set/get/list/rm` | Blocked on Rust |
| File explorer | `modules/explorer/lib/useFileTree.ts`, `lib/contextActions.ts`, `ExplorerSearch.tsx` | `fs.read_dir`, `fs.search`, `fs.move`, `fs.delete` | Blocked on Rust |
| Editor | `modules/editor/lib/useDocument.ts`, `NewEditorDialog.tsx` | `fs.read_file`, `fs.write_file` | Blocked on Rust |
| Terminal | `modules/terminal/lib/pty-bridge.ts`, `useTerminalSession.ts` | `pty.open` (WS), `pty.write`, `pty.resize` | Blocked on Rust |
| Status bar | `modules/statusbar/CwdBreadcrumb.tsx` | `fs.cwd` (or config-source) | Blocked on Rust |
| Settings window mgmt | `modules/settings/openSettingsWindow.ts`, `settings/main.tsx` | in-app routing (browser/mobile) + shell-injected adapter (desktop) | Shell-injection |
| Window chrome | `app/App.tsx`, `components/WindowControls.tsx`, `lib/platform.ts`, `modules/preview/PreviewAddressBar.tsx`, `modules/updater/useUpdater.ts` | shell-injected capability interfaces | Shell-injection |

The legitimate target is **zero `@tauri-apps/*` imports outside
`src/shells/<shell>/`**. The shell entries (`src/shells/desktop/main.tsx`
specifically) are the *one* place Tauri APIs may be touched directly —
that's the seam where the `TauriIpcClient` and desktop capability
adapters get constructed.

## Where new surfaces fit

SCOPE.md Phase 2 also calls for surfaces Terax does not have:
repo-grouped jobs dashboard, per-job stage/task timeline, review
approval card. These are real product surfaces, not architectural
plumbing.

They live in `codeless/ui/codeless-ui/src/modules/jobs/` (and
peer modules for review/etc.) and **mount inside the Terax shell**
— a new tab, sidebar entry, or panel in the existing layout. They do
**not** replace `<App />`. A `?view=jobs` debug route that bypasses
the shell is acceptable as a developer scaffold; ship-quality wiring
mounts inside.

The jobs surfaces can be built today against the existing 7 RPC
methods + `MockRpcClient`. Their visual design is independent of the
Tauri-conversion grind, so they can run in parallel.

## Ground rules

- **Components never import `@tauri-apps/*`** (R2 in workspace
  `CLAUDE.md`). Every Tauri call hides behind `RpcClient` or a
  shell-injected capability adapter. Trip this rule and a tick halts.
- **Components never import `fetch` against the codeless-server.**
  That is `HttpSseClient`'s job.
- **Per-shell files are forbidden.** No `Foo.web.tsx`. No
  `Foo.mobile.tsx`. Responsive design + injected capability
  interfaces cover every per-platform difference. If a Tauri-mobile
  feature genuinely doesn't work, write a thin native Tauri plugin —
  not a parallel UI.
- **Shells own only their entry file.** `src/shells/<shell>/main.tsx`
  constructs the `RpcClient` and capability adapters and mounts
  `<App />`. Anything beyond that is a smell.

## Pointers

- Project scope: [`SCOPE.md`](./SCOPE.md)
- Per-file conversion list: [`UI-PORT-AUDIT.md`](./UI-PORT-AUDIT.md)
- Workspace agent rules: [`../CLAUDE.md`](../CLAUDE.md)
- Inner-repo agent rules: [`../codeless/CLAUDE.md`](../codeless/CLAUDE.md)
- UI package: [`../codeless/ui/codeless-ui/`](../codeless/ui/codeless-ui/)
- Reusable RPC layer: [`../codeless/ui/codeless-ui/src/lib/rpc/`](../codeless/ui/codeless-ui/src/lib/rpc/)
