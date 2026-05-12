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
pinned SHA.

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

SCOPE.md estimated "~35 files". Actual numbers at the pinned SHA:

- **31 files import any `@tauri-apps/*` API or plugin** — total surface
  to refactor through the `RpcClient` boundary or shell-injected
  interfaces (clipboard, file picker, biometric).
- **11 files import `@tauri-apps/api/core`** specifically (the R2
  banned import):

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

## Suggested Phase 2 stage skeleton

Not a contract — feeds the Phase 2 session doc. Stages 0–2 + the
`MockRpcClient`-friendly bits of stage 6 are landed; the file-by-file
conversion grind is the bulk of what remains. Stage order reflects
unblock-by-RPC: surfaces whose RPC methods exist today come first;
surfaces blocked on `RpcServer` additions wait for them.

**Landed (Phase 2 stage 0–2 equivalents):**

- [x] 0. Upstream tree copied into `codeless/ui/codeless-ui/`,
  strings renamed terax→codeless, `NOTICE.md` + `LICENSE` in place,
  `pnpm dev` builds, `pnpm exec tsc --noEmit` clean.
- [x] 1. `RpcClient` interface + `HttpSseClient` + `MockRpcClient` +
  `<RpcProvider>` + `useRpc()`/`useRepos()`/`useJobs()`/`useJob()`/
  `useEventStream()` hooks. Bearer-token + base-URL config.
- [x] 2. `src/components/ai-elements/` already lifted (no Tauri
  imports — landed with the initial copy).

**Available now — RPC method exists in `codeless-rpc`:**

- [S] 6. **AI chat panel surface.** Replace `src/modules/ai/lib/agent.ts`
  (Vercel AI SDK in-browser loop) with `useRpc().call("submit_job", …)`
  + `useEventStream({scope:"job", job_id}, …)` rendering `ai-token`
  deltas as they arrive. Strip `composer.tsx` / `lib/native.ts` Tauri
  imports. Keep tool *schemas* in `src/modules/ai/tools/*` for UI
  rendering; delete the executor code (tools run in Rust).
- [S] 8. **Repo-grouped jobs dashboard** (new — no upstream
  equivalent). *Code already in `src/modules/jobs/`; needs to mount
  inside `<App />` as a tab/sidebar entry, not replace it.*
- [S] 9. **Per-job stage/task timeline** (new). *Code already in
  `src/modules/jobs/JobTimeline.tsx`; mounts inside the same dashboard
  surface.*
- [S] 9b. **`TauriIpcClient`** — second `RpcClient` impl. Mechanical TS
  work; keeps the desktop shell viable from day one without forcing a
  conversion of every Terax file before the Tauri host can run.

**Blocked on Rust `RpcServer` additions:**

- [S] 7. Settings → provider keys (`ProviderKeyCard.tsx`,
  `ModelsSection.tsx`, `lib/keyring.ts`) — needs
  `RpcServer::secrets_{set,get,list,rm}`. The Rust impl exists in
  `codeless-adapters-host::secrets`; only the trait method + transport
  binding is missing.
- [M] 4. File explorer (`useFileTree.ts`, `contextActions.ts`,
  `ExplorerSearch.tsx`) — needs `RpcServer::fs_{read_dir,search,move,
  delete}`.
- [M] 3. Editor (`useDocument.ts`, `NewEditorDialog.tsx`) — needs
  `RpcServer::fs_{read_file,write_file}`.
- [M] 5. Terminal (`pty-bridge.ts`, `useTerminalSession.ts`) — needs
  WebSocket-backed `RpcServer::pty_*` (or a separate `PtyClient`
  surface — open question). PTY is the only RPC method that doesn't
  fit the request/reply shape; it earns its own transport.
- [S] Status bar (`CwdBreadcrumb.tsx`) — needs `RpcServer::fs_cwd` or
  a config-source equivalent.

**Shell-injection (no RPC, capability adapter pattern):**

- [S] Settings window mgmt (`openSettingsWindow.ts`, `settings/main.tsx`)
  — multi-window is Tauri-only; replace with in-app routing for browser
  / mobile, keep a shell-injected adapter for desktop.
- [S] Window chrome (`WindowControls.tsx`, `lib/platform.ts`,
  `app/App.tsx` Tauri bits, `modules/preview/PreviewAddressBar.tsx`,
  `modules/updater/useUpdater.ts`) — capability-adapter interfaces, one
  per concern. Browser/mobile shells inject no-op or web-equivalent
  implementations.

**Wrap-up:**

- [S] Audit for any leftover `terax` strings, delete
  `/tmp/upstream-ui-ref`, confirm zero `@tauri-apps/api/core` imports
  under `codeless/ui/codeless-ui/src/` outside the desktop-shell entry.
- [S] Wire the specta codegen output from `codeless-types` into the
  UI build so `wire.ts` is generated, not maintained.
