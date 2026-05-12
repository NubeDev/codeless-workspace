# UI port audit

Reference: upstream `crynta/terax-ai` @ commit `a628d62db1bfabf44085aeca992689ff5c4c6224`,
license Apache-2.0.

This is the per-file worklist for converting the upstream-derived
Terax UI to talk to `codeless-runtime` through the `RpcClient`
interface. The architectural rationale lives in
[`UI-ARCHITECTURE.md`](./UI-ARCHITECTURE.md) — read that first if
you're new to the boundary.

The full upstream tree was already copied into
`codeless/ui/codeless-ui/` (with strings renamed terax→codeless and
attribution preserved in `NOTICE.md`). The remaining work is the
conversion grind: each file that imports `@tauri-apps/*` is rewired
through `RpcClient.*` (typed RPC) or a shell-injected capability
adapter (clipboard, file picker, biometric).

## What "done" looks like per file

1. No imports from `@tauri-apps/api/*` or `@tauri-apps/plugin-*`.
2. All transport calls go through `useRpc()` (or a capability adapter
   passed in as a prop).
3. Type-check + build still pass; the surface still renders against
   `MockRpcClient`.
4. Renamed strings preserved (no `Terax`/`terax`/`TERAX` outside
   `NOTICE.md`).

## Reuse list (with concrete upstream paths)

Maps SCOPE.md's "Reuse (visual layer)" bullets to actual files at the
pinned SHA. Overlaps with the surface table in
[`UI-ARCHITECTURE.md`](./UI-ARCHITECTURE.md); kept separately because
the value here is the *concrete upstream paths* — useful when
diffing against upstream during the conversion grind.

| SCOPE bullet | Upstream source |
|---|---|
| CodeMirror 6 editor + theming | `src/modules/editor/` (8 files), CodeMirror deps in `package.json` |
| xterm.js terminal component | `src/modules/terminal/` — replace `lib/pty-bridge.ts` with WS transport |
| File explorer React component | `src/modules/explorer/` — replace `lib/contextActions.ts`, `useFileTree.ts`, `ExplorerSearch.tsx` IPC paths |
| AI message / streaming markdown rendering | `src/components/ai-elements/` (9 files: `conversation`, `message`, `markdown-code`, `code-block`, `reasoning`, `shimmer`, `snippet`, `tool`, `context`) |
| Approval-card UI | `src/modules/ai/components/AiToolApproval.tsx`, `PlanDiffReview.tsx` |
| Settings shell, providers form layout | `src/settings/` — replace `ProviderKeyCard.tsx`, `ModelsSection.tsx` IPC; lift `SettingsApp.tsx` shell |
| Tailwind + shadcn/ui setup | `components.json`, `vite.config.ts`, `tsconfig.json`, `src/components/ui/` (38 shadcn primitives), `src/styles/` |
| Project-memory concept | `TERAX.md` → already mapped to `codeless/CODELESS.md` (Phase 1 stage 7) |

## Replace list (do not port — already covered by Rust core)

These exist in the upstream tree. Do not `cp` them; the Rust core
provides the equivalent over `RpcClient`.

- `src/modules/ai/lib/agent.ts` — AI SDK loop, replaced by `codeless-runtime`
- `src/modules/ai/lib/transport.ts` — replaced by `codeless-rpc`
- `src/modules/ai/lib/native.ts` — direct Tauri invokes, replaced by `RpcClient`
- `src/modules/ai/lib/keyring.ts` — replaced by `RpcSecrets` (host + hosted backends)
- `src/modules/ai/lib/sessions.ts`, `agents.ts`, `snippets.ts`,
  `todos.ts` — Zustand stores backed by Tauri store plugin; replace
  with RPC-backed equivalents
- `src/modules/ai/tools/*` (`context.ts`, `edit.ts`, `fs.ts`, `search.ts`,
  `shell.ts`, `subagent.ts`, `terminal.ts`, `todo.ts`, `tools.ts`) —
  tools execute in Rust now; TS keeps only the schema for UI rendering
- `src/modules/ai/store/agentsStore.ts`, `snippetsStore.ts` — replaced
  by RPC-subscribed stores
- `src/modules/updater/` — re-evaluate for hosted mode (SCOPE "Drop")
- `src-tauri/` — host-side Rust modules (`pty`, `shell`, `fs`) inform
  Phase 6 `codeless-adapters-host` design; do not lift verbatim

## Tauri-coupled files (R2 violation surface)

Counts at the current tree (excluding `*.md`, `src/shells/<shell>/*`
which are legitimate injection points per
[`UI-ARCHITECTURE.md`](./UI-ARCHITECTURE.md), and a doc-comment
false-positive in `src/lib/rpc/client.ts:5` that mentions
`@tauri-apps/api/*` in prose):

- **31 files import any `@tauri-apps/*` API or plugin** — the surface
  to refactor through the `RpcClient` boundary or shell-injected
  interfaces (clipboard, file picker, biometric).
- **11 files import `@tauri-apps/api/core`** specifically (the R2
  banned import).
- **The conversion target is zero outside `src/shells/<shell>/`.** The
  shell entry files are *expected* to import `@tauri-apps/*`
  directly — that is where the `TauriIpcClient` and desktop
  capability adapters are constructed. The desktop entry
  (`src/shells/desktop/main.tsx`) currently does so by design.

The 11 `api/core` files (the conversion priority list):

```
src/modules/ai/lib/composer.tsx
src/modules/ai/lib/keyring.ts
src/modules/ai/lib/native.ts
src/modules/editor/lib/useDocument.ts
src/modules/editor/NewEditorDialog.tsx
src/modules/explorer/ExplorerSearch.tsx
src/modules/explorer/lib/useFileTree.ts
src/modules/settings/openSettingsWindow.ts
src/modules/statusbar/CwdBreadcrumb.tsx
src/modules/terminal/lib/pty-bridge.ts
src/settings/sections/ModelsSection.tsx
```

The full 31-file list (broader Tauri coupling, including plugins that
become shell-injected interfaces under R3):

```
src/app/App.tsx
src/components/WindowControls.tsx
src/lib/platform.ts
src/main.tsx
src/modules/ai/lib/agents.ts
src/modules/ai/lib/composer.tsx
src/modules/ai/lib/keyring.ts
src/modules/ai/lib/native.ts
src/modules/ai/lib/sessions.ts
src/modules/ai/lib/snippets.ts
src/modules/ai/lib/todos.ts
src/modules/ai/store/agentsStore.ts
src/modules/ai/store/snippetsStore.ts
src/modules/editor/lib/useDocument.ts
src/modules/editor/NewEditorDialog.tsx
src/modules/explorer/ExplorerSearch.tsx
src/modules/explorer/lib/contextActions.ts
src/modules/explorer/lib/useFileTree.ts
src/modules/preview/PreviewAddressBar.tsx
src/modules/settings/openSettingsWindow.ts
src/modules/settings/store.ts
src/modules/statusbar/CwdBreadcrumb.tsx
src/modules/terminal/lib/pty-bridge.ts
src/modules/terminal/lib/useTerminalSession.ts
src/modules/updater/useUpdater.ts
src/settings/components/ProviderKeyCard.tsx
src/settings/main.tsx
src/settings/sections/AboutSection.tsx
src/settings/sections/GeneralSection.tsx
src/settings/sections/ModelsSection.tsx
src/settings/SettingsApp.tsx
```

## Phase 2 work groups

Not a contract — feeds the Phase 2 session doc. Grouped by
unblock-status rather than numbered, because order is determined by
which RPC methods exist today.

### Landed

- [x] **Upstream tree in place.** `codeless/ui/codeless-ui/` copied,
  strings renamed terax→codeless, `NOTICE.md` + `LICENSE` in place,
  `pnpm dev` builds, `pnpm exec tsc --noEmit` clean.
- [x] **RPC boundary layer.** `RpcClient` interface, `HttpSseClient`,
  `MockRpcClient`, `<RpcProvider>`, `useRpc()` / `useRepos()` /
  `useJobs()` / `useJob()` / `useEventStream()` hooks. Bearer-token
  + base-URL config. Lives in `src/lib/rpc/`.
- [x] **`ai-elements`.** `src/components/ai-elements/` lifted with no
  Tauri imports.
- [x] **Jobs dashboard scaffolding** (new — no upstream equivalent).
  Lives in `src/modules/jobs/` (`JobsDashboard`, `JobRow`, `JobDetail`,
  `JobTimeline`, `SubmitJobDialog`, `StatusBadge`). Currently rendered
  via a `?view=jobs` debug shortcut; ship-quality wiring mounts inside
  `<App />` as a new tab/sidebar entry (still to do).

### Available now — RPC method exists in `codeless-rpc`

- [M] **AI chat panel conversion.** Replace `src/modules/ai/lib/agent.ts`
  (Vercel AI SDK in-browser loop) with `useRpc().call("submit_job", …)`
  + `useEventStream({scope:"job", job_id}, …)` rendering `ai-token`
  deltas as they arrive. Strip `composer.tsx` / `lib/native.ts` Tauri
  imports. Keep tool *schemas* in `src/modules/ai/tools/*` for UI
  rendering; delete the executor code (tools run in Rust).
- [S] **Mount jobs dashboard inside `<App />`.** Pick a tab / sidebar
  slot in the Terax shell layout, drop the `?view=jobs` shortcut.
- [S] **`TauriIpcClient`.** Second `RpcClient` impl wrapping `invoke()`
  and Tauri events. Mechanical TS work; keeps the desktop shell viable
  from day one rather than waiting on the full conversion grind.

### Blocked on Rust `RpcServer` additions

- [S] **Settings → provider keys** (`ProviderKeyCard.tsx`,
  `ModelsSection.tsx`, `lib/keyring.ts`) — needs
  `RpcServer::secrets_{set,get,list,rm}`. The Rust impl already exists
  in `codeless-adapters-host::secrets`; only the trait method +
  transport binding is missing.
- [M] **File explorer** (`useFileTree.ts`, `contextActions.ts`,
  `ExplorerSearch.tsx`) — needs `RpcServer::fs_{read_dir,search,move,delete}`.
- [M] **Editor** (`useDocument.ts`, `NewEditorDialog.tsx`) — needs
  `RpcServer::fs_{read_file,write_file}`.
- [M] **Terminal** (`pty-bridge.ts`, `useTerminalSession.ts`) — needs
  WebSocket-backed `RpcServer::pty_*` (or a separate `PtyClient`
  surface — open question). PTY is the only RPC method that doesn't
  fit the request/reply shape; it earns its own transport.
- [S] **Status bar** (`CwdBreadcrumb.tsx`) — needs `RpcServer::fs_cwd`
  or a config-source equivalent.

### Shell-injection (no RPC, capability adapter pattern)

- [S] **Settings window mgmt** (`openSettingsWindow.ts`,
  `settings/main.tsx`) — multi-window is Tauri-only; replace with
  in-app routing for browser / mobile, keep a shell-injected adapter
  for desktop.
- [S] **Window chrome** (`WindowControls.tsx`, `lib/platform.ts`,
  `app/App.tsx` Tauri bits, `modules/preview/PreviewAddressBar.tsx`,
  `modules/updater/useUpdater.ts`) — capability-adapter interfaces,
  one per concern. Browser/mobile shells inject no-op or web-equivalent
  implementations.

### Wrap-up

- [S] Wire the specta codegen output from `codeless-types` into the UI
  build so `wire.ts` and `methods.ts` are generated, not hand-mirrored.
- [S] Audit pass: zero `@tauri-apps/*` imports outside
  `src/shells/<shell>/`; zero `terax` strings outside `NOTICE.md`.
