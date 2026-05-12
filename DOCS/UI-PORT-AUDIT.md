# UI port audit

Reference: upstream `crynta/terax-ai` @ commit `a628d62db1bfabf44085aeca992689ff5c4c6224`,
license Apache-2.0.

The upstream codebase is the source of UI components for `codeless-ui`
(Phase 2). We do **not** maintain a parallel fork or a separate NubeDev
repo — the destination is `codeless/ui/codeless-ui/` inside the inner
`codeless/` repo, and the upstream name does not survive into the
codebase. Attribution lives in `codeless/ui/codeless-ui/NOTICE.md`
(Apache-2.0 requirement).

## Port mechanics

1. `git clone --depth 50 https://github.com/crynta/terax-ai /tmp/upstream-ui-ref`
   — scratch reference checkout, deleted when the port is done.
2. Per port stage, `cp` the specific files listed below into the
   matching subtree under `codeless/ui/codeless-ui/src/…`.
3. In the same commit, strip Tauri coupling (see "Tauri-coupled files"
   below) and rename any `terax`/`Terax`/`TERAX` identifiers to
   `codeless`. The end state contains zero `terax` strings outside
   `NOTICE.md`.

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

Not a contract — feeds the Phase 2 session doc when Phase 1 wraps.

- [S] 0. Scaffold `codeless/ui/codeless-ui/` (Vite + React 19 + TS +
  Tailwind v4 + shadcn). Lift `components.json`, `vite.config.ts`,
  `tsconfig.json`, `src/components/ui/`, `src/styles/`. No Tauri deps.
- [S] 1. Define `RpcClient` TS interface + `HttpSseClient`
  implementation (consumes specta-generated wire types from stage 3).
- [S] 2. Lift `src/components/ai-elements/` verbatim (no Tauri imports).
- [M] 3. Port `src/modules/editor/` — replace `useDocument.ts` and
  `NewEditorDialog.tsx` IPC with `RpcClient.fs.*` calls.
- [M] 4. Port `src/modules/explorer/` — replace `useFileTree`,
  `ExplorerSearch`, `contextActions` IPC.
- [M] 5. Port `src/modules/terminal/` — replace `pty-bridge` with WS
  transport via `RpcClient.pty.open()`.
- [S] 6. Port AI chat panel shell (`src/modules/ai/components/AiChat.tsx`,
  `AiInputBar.tsx`, `AiToolApproval.tsx`, `PlanDiffReview.tsx`) —
  keeping only schemas from `src/modules/ai/tools/*`, deleting the
  loop/transport/native/keyring/tools.
- [S] 7. Settings shell — port `src/settings/SettingsApp.tsx` +
  sections, replace `ProviderKeyCard.tsx` and `ModelsSection.tsx` IPC
  with `RpcClient.secrets.*` and `RpcClient.config.*`.
- [S] 8. Repo-grouped jobs dashboard (new — no upstream equivalent).
- [S] 9. Per-job stage/task timeline (new).
- [S] 10. Wrap-up: `NOTICE.md`, delete `/tmp/upstream-ui-ref`, audit
  for any leftover `terax` strings, confirm zero `@tauri-apps/api/core`
  imports under `codeless/ui/codeless-ui/`.
